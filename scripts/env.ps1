# Puts the Rust (GNU) toolchain and MinGW on PATH for this shell.
# Usage:  . .\scripts\env.ps1
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
$wl = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Directory -ErrorAction SilentlyContinue |
    Where-Object Name -like 'BrechtSanders.WinLibs.POSIX.MSVCRT*' | Select-Object -First 1
$mingwBin = if ($wl) { Join-Path $wl.FullName 'mingw64\bin' } else { $null }
$parts = @($cargoBin)
if ($mingwBin -and (Test-Path $mingwBin)) { $parts += $mingwBin } else { Write-Warning 'MinGW (WinLibs) not found' }
$env:Path = (($parts + ($env:Path -split ';')) | Where-Object { $_ } | Select-Object -Unique) -join ';'
$env:CARGO_TARGET_DIR = $null
