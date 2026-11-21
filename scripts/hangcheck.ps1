# Reports whether MEDORE's WebView2 browser windows answer messages.
param([string]$Process = 'medore')
if (-not ('HW2' -as [type])) {
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class HW2 {
  public delegate bool CB(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(CB cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, CB cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern IntPtr SendMessageTimeout(IntPtr h, uint msg, IntPtr w, IntPtr l, uint flags, uint timeout, out IntPtr res);
  public static string Check(uint pid) {
    int total = 0, dead = 0;
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid) EnumChildWindows(h, (c, l2) => {
        uint cp; GetWindowThreadProcessId(c, out cp);
        if (cp != pid) { total++; IntPtr r; if (SendMessageTimeout(c, 0, IntPtr.Zero, IntPtr.Zero, 2, 1000, out r) == IntPtr.Zero) dead++; }
        return true; }, IntPtr.Zero);
      return true; }, IntPtr.Zero);
    return dead == 0 ? ("ok (" + total + " web windows answer)") : ("HUNG (" + dead + "/" + total + " web windows silent)");
  }
}
"@
}
$p = Get-Process -Name $Process -ErrorAction Stop | Select-Object -First 1
[HW2]::Check([uint32]$p.Id)
