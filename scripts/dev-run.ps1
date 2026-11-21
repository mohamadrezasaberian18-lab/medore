# Build (debug, embedded frontend) and start MEDORE against a throwaway
# profile, with WebView2 remote debugging on port 9333 for scripts/cdp.mjs.
#   .\scripts\dev-run.ps1 [-NoBuild] [-Home <dir>] [-Files a.mkv,b.mkv]
param(
    [switch]$NoBuild,
    [string]$Home2 = (Join-Path $env:TEMP 'medore-test'),
    [string[]]$Files = @(),
    [switch]$Release
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
. (Join-Path $PSScriptRoot 'env.ps1')
Get-Process medore -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 300
$profile = if ($Release) { 'release' } else { 'debug' }
if (-not $NoBuild) {
    Push-Location $root
    npx vite build --logLevel error
    Pop-Location
    Push-Location (Join-Path $root 'src-tauri')
    $args2 = @('build', '--features', 'tauri/custom-protocol')
    if ($Release) { $args2 += '--release' }
    cargo @args2 2>&1 | Select-String -Pattern '^error|error\[|-->|Finished'
    Pop-Location
}
$bin = Join-Path $root "src-tauri\target\$profile"
foreach ($f in @('vendor\mpv\libmpv-2.dll', 'src-tauri\icons\medore-dark.ico', 'src-tauri\icons\medore-light.ico')) {
    $src = Join-Path $root $f
    $dst = Join-Path $bin (Split-Path $f -Leaf)
    if (-not (Test-Path $dst) -or (Get-Item $src).LastWriteTime -gt (Get-Item $dst).LastWriteTime) { Copy-Item $src $dst -Force }
}
$env:MEDORE_HOME = $Home2
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9333'
$p = Start-Process (Join-Path $bin 'medore.exe') -ArgumentList $Files -PassThru -WindowStyle Normal
"started pid $($p.Id)"
