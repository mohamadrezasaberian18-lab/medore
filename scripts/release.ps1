# Build the MEDORE installer (NSIS) and the signed update files.
#   .\scripts\release.ps1
# Needs the update-signing key at %USERPROFILE%\.tauri\medore-updater.key
# (never commit it; without it existing installs cannot accept updates).
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
. (Join-Path $PSScriptRoot 'env.ps1')
$key = Join-Path $env:USERPROFILE '.tauri\medore-updater.key'
if (-not (Test-Path $key)) { throw "signing key missing: $key" }
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $key -Raw
if ($null -eq $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) { $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = '""' }  # the key was created with the password ""
Push-Location $root
try {
    node scripts/render-icons.mjs | Out-Null
    npx tauri build --bundles nsis 2>&1 | ForEach-Object { $_ } | Select-String -Pattern 'error|warning: unused|Finished|Built application|bundle|Signature|\.exe|\.sig|Info' | ForEach-Object { $_.Line }
} finally { Pop-Location }
$out = Join-Path $root 'src-tauri\target\release\bundle\nsis'
Get-ChildItem $out -ErrorAction SilentlyContinue | Select-Object Name, @{n='MB';e={[math]::Round($_.Length/1MB,2)}}
