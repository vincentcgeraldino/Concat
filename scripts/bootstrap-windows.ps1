# bootstrap-windows.ps1
# Sets up the native dependencies Concat needs to build on Windows: an
# FFmpeg 9 development build (headers and import libraries) unpacked into
# %LOCALAPPDATA%\concat\ffmpeg-dev and pointed at with FFMPEG_DIR, the way
# src/README.md describes. ffmpeg-sys-the-third finds FFMPEG_DIR before it
# looks for pkg-config or vcpkg, so no package manager is needed; bindgen
# is what needs the libclang this also locates.
# Run from the repository root:  powershell -ExecutionPolicy Bypass -File scripts\bootstrap-windows.ps1

$ErrorActionPreference = "Stop"

function Info($msg) { Write-Host "[bootstrap] $msg" -ForegroundColor Cyan }
function Fail($msg) { Write-Host "[bootstrap] $msg" -ForegroundColor Red; exit 1 }

# Matches ffmpeg-sys-the-third 6.0.0+ffmpeg-9.0 in src/Cargo.lock. The
# asset name is BtbN's; "latest" tracks the 9.0 branch, not master.
$url = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n9.0-latest-win64-gpl-shared-9.0.zip"
$dest = Join-Path $env:LOCALAPPDATA "concat\ffmpeg-dev"

# --- FFmpeg development libraries ----------------------------------------
function Find-FFmpegRoot {
    if (-not (Test-Path $dest)) { return $null }
    Get-ChildItem $dest -Directory |
        Where-Object { Test-Path (Join-Path $_.FullName "include\libavcodec") } |
        Select-Object -First 1
}

$root = $null
if ($env:FFMPEG_DIR -and (Test-Path (Join-Path $env:FFMPEG_DIR "include\libavcodec"))) {
    $root = Get-Item $env:FFMPEG_DIR
    Info "FFMPEG_DIR already points at $($root.FullName)"
} else {
    $root = Find-FFmpegRoot
}

if (-not $root) {
    Info "Downloading the FFmpeg 9.0 shared build (BtbN)..."
    New-Item -ItemType Directory -Force $dest | Out-Null
    $zip = Join-Path $env:TEMP "ffmpeg-n9.0-latest-win64-gpl-shared-9.0.zip"
    if (Get-Command curl.exe -ErrorAction SilentlyContinue) {
        & curl.exe -L --fail --output $zip $url
        if ($LASTEXITCODE -ne 0) { Fail "download failed" }
    } else {
        $ProgressPreference = "SilentlyContinue"
        Invoke-WebRequest -Uri $url -OutFile $zip
    }
    Info "Unpacking into $dest ..."
    if (Get-Command tar.exe -ErrorAction SilentlyContinue) {
        & tar -xf $zip -C $dest
    } else {
        Expand-Archive -Path $zip -DestinationPath $dest
    }
    $root = Find-FFmpegRoot
    if (-not $root) { Fail "the archive did not unpack an FFmpeg tree into $dest" }
}

$env:FFMPEG_DIR = $root.FullName
Info "FFMPEG_DIR=$($root.FullName)"
if (-not (Test-Path (Join-Path $root.FullName "lib\avcodec.lib"))) {
    Info "no avcodec.lib under lib\; the linker needs the import libraries a shared build ships"
}
# The DLLs are found at run time the same way the headers are at build
# time: by asking, not by guessing.
$binPath = Join-Path $root.FullName "bin"
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$binPath*") {
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$binPath", "User")
    Info "FFmpeg's DLLs added to the user PATH: a run finds them without the script."
}

# --- libclang, for bindgen ------------------------------------------------
$libclang = $env:LIBCLANG_PATH
if (-not ($libclang -and (Test-Path (Join-Path $libclang "libclang.dll")))) {
    $llvm = "C:\Program Files\LLVM\bin"
    if (Test-Path (Join-Path $llvm "libclang.dll")) {
        $env:LIBCLANG_PATH = $llvm
        Info "LIBCLANG_PATH=$llvm"
    } else {
        Fail "libclang.dll not found. Install LLVM (winget install LLVM.LLVM) and set LIBCLANG_PATH to its bin directory."
    }
}

# --- verify ---------------------------------------------------------------
if ([Environment]::GetEnvironmentVariable("FFMPEG_DIR", "User") -ne $root.FullName) {
    [Environment]::SetEnvironmentVariable("FFMPEG_DIR", $root.FullName, "User")
    Info "FFMPEG_DIR saved to the user environment: new shells will find it."
}
Info "Verifying the FFmpeg link with cargo check -p concat-media..."
Push-Location (Join-Path $PSScriptRoot "..\src")
try {
    cargo check -p concat-media
    if ($LASTEXITCODE -ne 0) { Fail "cargo check failed" }
    Info "FFmpeg links. Checking the whole window (this takes a while)..."
    cargo check -p concat
    if ($LASTEXITCODE -ne 0) { Fail "cargo check failed" }
    Info "Build OK."
} finally {
    Pop-Location
}
