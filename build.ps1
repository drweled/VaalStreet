# Builds Vaal Street:
#   1. the page (app\) into ui\
#   2. the program and its plain installer (src-tauri)
#   3. Vaal Street Setup (setup\): the pixel installer people download. It
#      carries the plain installer inside and runs it quietly.
# Results go to dist\: VaalStreet_<version>_Setup.exe, its updater signature
# and latest.json (the updater downloads the same Setup file).
#
#   powershell -ExecutionPolicy Bypass -File build.ps1            # full build
#   powershell -ExecutionPolicy Bypass -File build.ps1 -UiOnly    # the page only
#   powershell -ExecutionPolicy Bypass -File build.ps1 -NoUpdater # no update signing
#
# Releases are signed for the updater with the maker's private key, kept in
# %USERPROFILE%\.vaalstreet-keys\vaalstreet-updater.key (never in this
# repository). Without that key, build with -NoUpdater: the program works, it
# just can't be offered as an update to installed copies.
# The npm and cargo work happens outside the project folder (%USERPROFILE%\
# .vaalstreet-target), so node_modules and target never sync anywhere.
param([switch]$SkipTypes, [switch]$UiOnly, [switch]$NoUpdater, [string]$Owner = '')
$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$source = Join-Path $root 'app'
$ui = Join-Path $root 'ui'
$target = Join-Path $env:USERPROFILE '.vaalstreet-target'
$work = Join-Path $target 'app-build'
$key = Join-Path $env:USERPROFILE '.vaalstreet-keys\vaalstreet-updater.key'

# ------------------------------------------------------------------ 1. page
New-Item -ItemType Directory -Force $work, $ui | Out-Null
robocopy $source $work /MIR /XD node_modules /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -ge 8) { throw "copying the source failed ($LASTEXITCODE)" }

Push-Location $work
try {
  if (-not (Test-Path node_modules\.package-lock.json) -or
      (Get-Item package-lock.json).LastWriteTime -gt (Get-Item node_modules\.package-lock.json).LastWriteTime) {
    npm ci --no-audit --no-fund
    if ($LASTEXITCODE) { throw 'npm ci failed' }
  }
  if (-not $SkipTypes) {
    npx vue-tsc --noEmit
    if ($LASTEXITCODE) { throw 'type check failed' }
  }
  $env:VAAL_UI = (Resolve-Path $ui).Path
  npx vite build
  if ($LASTEXITCODE) { throw 'vite build failed' }
} finally {
  Remove-Item Env:\VAAL_UI -ErrorAction SilentlyContinue
  Pop-Location
}
Write-Host "Vaal Street page built into $ui"
if ($UiOnly) { return }

$conf = Get-Content (Join-Path $root 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$version = $conf.version
$signed = (-not $NoUpdater) -and (Test-Path $key)
if (-not $NoUpdater -and -not $signed) {
  Write-Warning "No updater key at ${key} - building without update signing (-NoUpdater)."
}
$env:CARGO_TARGET_DIR = Join-Path $target 'cargo'

# --------------------------------------------- 2. program + plain installer
Push-Location (Join-Path $root 'src-tauri')
try {
  cargo tauri build
  if ($LASTEXITCODE) { throw 'cargo tauri build failed' }
} finally {
  Pop-Location
}
$bundle = Join-Path $target "cargo\release\bundle\nsis"
$plain = Get-ChildItem (Join-Path $bundle "*_${version}_x64-setup.exe") | Sort-Object LastWriteTime | Select-Object -Last 1
if (-not $plain) { throw 'the plain installer was not made' }

# ------------------------------------------------ 3. Vaal Street Setup
$setupDir = Join-Path $root 'setup'
Copy-Item (Join-Path $source 'public\fonts\vt323.ttf') (Join-Path $setupDir 'ui\vt323.ttf') -Force
Copy-Item (Join-Path $root 'src-tauri\icons\128x128.png') (Join-Path $setupDir 'ui\logo.png') -Force
$empty = Join-Path $target 'empty.bin'
[IO.File]::WriteAllBytes($empty, [byte[]]@())
$uninstaller = Join-Path $target 'uninstaller.exe'
Push-Location $setupDir
try {
  # first the uninstaller: the same program with nothing carried inside
  $env:VS_PAYLOAD = $empty
  $env:VS_UNINSTALLER = $empty
  cargo tauri build --no-bundle
  if ($LASTEXITCODE) { throw 'building the uninstaller failed' }
  Copy-Item (Join-Path $target 'cargo\release\VaalStreetSetup.exe') $uninstaller -Force
  # then the setup, carrying the installer and the uninstaller
  $env:VS_PAYLOAD = $plain.FullName
  $env:VS_UNINSTALLER = $uninstaller
  cargo tauri build --no-bundle
  if ($LASTEXITCODE) { throw 'building Vaal Street Setup failed' }
} finally {
  Remove-Item Env:\VS_PAYLOAD, Env:\VS_UNINSTALLER -ErrorAction SilentlyContinue
  Pop-Location
}
Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue

# dist\: the Setup under a name without spaces (GitHub keeps it as is), its
# signature, and latest.json for the updater
$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Force $dist | Out-Null
Get-ChildItem $dist -File | Remove-Item -Force
$name = "VaalStreet_${version}_Setup.exe"
$setup = Join-Path $dist $name
Copy-Item (Join-Path $target 'cargo\release\VaalStreetSetup.exe') $setup -Force
Write-Host "Setup: $setup"

if ($signed) {
  $env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content $key -Raw).Trim()
  try {
    # the key has no password ("--password=": PowerShell drops an empty '' argument)
    cargo tauri signer sign --password= $setup
    if ($LASTEXITCODE) { throw 'signing for the updater failed' }
  } finally {
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
  }
  if (-not $Owner) {
    $remote = git -C $root remote get-url origin 2>$null
    if ($remote -match 'github\.com[:/]([^/]+)/') { $Owner = $Matches[1] }
  }
  if (-not $Owner) { throw 'GitHub owner unknown: pass -Owner <your GitHub name>' }
  $signature = (Get-Content "$setup.sig" -Raw).Trim()
  $latest = [ordered]@{
    version   = $version
    notes     = "Vaal Street $version"
    pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    platforms = [ordered]@{
      'windows-x86_64' = [ordered]@{
        signature = $signature
        url       = "https://github.com/$Owner/VaalStreet/releases/download/v$version/$name"
      }
    }
  }
  # plain UTF-8 without a BOM: the updater's JSON reader refuses a BOM
  [IO.File]::WriteAllText((Join-Path $dist 'latest.json'), ($latest | ConvertTo-Json -Depth 5), (New-Object Text.UTF8Encoding $false))
  Write-Host "Updater file: $(Join-Path $dist 'latest.json')"
}
