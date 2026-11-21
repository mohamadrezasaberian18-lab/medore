# Wait Chain Traversal: what are the hung WebView2 threads waiting for?
param([string]$Process = 'medore')
if (-not ('WCT' -as [type])) {
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class WCT {
  [StructLayout(LayoutKind.Sequential)] struct LOCK { [MarshalAs(UnmanagedType.ByValArray, SizeConst=128)] public ushort[] Name; public long Timeout; public uint Alertable; }
  [StructLayout(LayoutKind.Sequential)] struct THR { public uint ProcessId, ThreadId, WaitTime, ContextSwitches; }
  [StructLayout(LayoutKind.Explicit, Size=280)] struct NODE {
    [FieldOffset(0)] public int ObjectType; [FieldOffset(4)] public int ObjectStatus;
    [FieldOffset(8)] public THR Thread; }
  [DllImport("advapi32.dll", SetLastError=true)] static extern IntPtr OpenThreadWaitChainSession(int flags, IntPtr cb);
  [DllImport("advapi32.dll", SetLastError=true)] static extern bool GetThreadWaitChain(IntPtr s, IntPtr ctx, int flags, uint tid, ref int count, [Out] NODE[] nodes, out int cycle);
  [DllImport("advapi32.dll")] static extern void CloseThreadWaitChainSession(IntPtr s);
  static readonly string[] Types = {"?","CriticalSection","SendMessage","Mutex","Alpc","Com","ThreadWait","ProcWait","Thread","ComActivation","Unknown","SocketIo","SmbIo"};
  public static string Chain(uint tid) {
    IntPtr s = OpenThreadWaitChainSession(0, IntPtr.Zero);
    var nodes = new NODE[16]; int count = 16; int cycle;
    bool ok = GetThreadWaitChain(s, IntPtr.Zero, 7, tid, ref count, nodes, out cycle);
    CloseThreadWaitChainSession(s);
    if (!ok) return "error " + Marshal.GetLastWin32Error();
    var sb = new StringBuilder();
    for (int i = 0; i < count; i++) {
      var n = nodes[i];
      string t = n.ObjectType >= 0 && n.ObjectType < Types.Length ? Types[n.ObjectType] : n.ObjectType.ToString();
      if (n.ObjectType == 8) sb.Append("[thread pid=" + n.Thread.ProcessId + " tid=" + n.Thread.ThreadId + " wait=" + n.Thread.WaitTime + "ms] ");
      else sb.Append("-> " + t + " (status " + n.ObjectStatus + ") ");
    }
    if (cycle != 0) sb.Append(" DEADLOCK CYCLE");
    return sb.ToString();
  }
}
"@
}
$pid0 = (Get-Process -Name $Process | Select-Object -First 1).Id
$browser = Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object { $_.ParentProcessId -eq $pid0 -and $_.CommandLine -notmatch '--type=' } | Select-Object -First 1
"medore pid $pid0, webview browser pid $($browser.ProcessId)"
foreach ($pr in @((Get-Process -Id $pid0), (Get-Process -Id $browser.ProcessId))) {
    foreach ($t in $pr.Threads) {
        if ($t.ThreadState -eq 'Wait' -and $t.WaitReason -in 'UserRequest', 'Executive', 'WrUserRequest', 'LpcReply', 'WrLpcReply') {
            $c = [WCT]::Chain([uint32]$t.Id)
            if ($c -match 'SendMessage|DEADLOCK|Alpc|Com') { "$($pr.ProcessName)/$($t.Id): $c" }
        }
    }
}
