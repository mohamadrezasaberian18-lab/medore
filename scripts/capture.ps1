# Capture a top-level window of a process (as composed on screen) to PNG.
# Usage: .\scripts\capture.ps1 -Process vesper -Out shot.png [-Front]
param(
    [string]$Process = 'vesper',
    [Parameter(Mandatory)][string]$Out,
    [switch]$Front,
    [int]$Crop = 0
)
Add-Type -AssemblyName System.Drawing
if (-not ('CapW' -as [type])) {
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class CapW {
  public delegate bool CB(IntPtr h, IntPtr l);
  [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Rt, B; }
  [DllImport("user32.dll")] public static extern bool EnumWindows(CB cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out R r, int size);
  public static IntPtr Biggest(uint pid) {
    IntPtr best = IntPtr.Zero; long area = 0;
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid && IsWindowVisible(h)) {
        R r; GetWindowRect(h, out r);
        long a = (long)(r.Rt - r.L) * (r.B - r.T);
        if (a > area) { area = a; best = h; }
      }
      return true;
    }, IntPtr.Zero);
    return best;
  }
}
"@
}
$procs = Get-Process -Name $Process -ErrorAction Stop
$h = [IntPtr]::Zero
foreach ($p in $procs) { $x = [CapW]::Biggest([uint32]$p.Id); if ($x -ne [IntPtr]::Zero) { $h = $x; break } }
if ($h -eq [IntPtr]::Zero) { throw "no visible window for $Process" }
if ($Front) {
    [CapW]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [CapW]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
    [CapW]::SetForegroundWindow($h) | Out-Null
    Start-Sleep -Milliseconds 350
}
$r = New-Object CapW+R
[CapW]::DwmGetWindowAttribute($h, 9, [ref]$r, 16) | Out-Null
$w = $r.Rt - $r.L; $hh = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
"captured ${w}x${hh} -> $Out"
