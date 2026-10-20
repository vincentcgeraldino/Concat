# Installs the Windows installers, and fails if the program does not
# arrive or does not start.
#
#   scripts/ci/smoke-windows-install.ps1 -Setup stage\Concat-<version>-windows-<arch>-setup.exe `
#                                        [-Msi  stage\Concat-<version>-windows-<arch>.msi]
#
# The build launches the staged folder before it packages it, but the
# installers themselves were never run: a shortcut pointing at nothing, a
# file left out of the package, a Start menu entry for a program that is
# not there - none of it showed up until somebody downloaded the file and
# found it did not work. 0.2.6 shipped a program that would not open; the
# bundle was checked, the installer was not.
#
# So the installers are installed here, the program is run from where the
# installer put it - not from the staged folder, which is what was
# checked - and the installer is then asked to take it away again, so the
# two installs do not tread on each other.
#
# The .exe is the one a person double-clicks; the .msi is the one an
# administrator deploys. Both are installed when both are given.
#
# The program is judged by its exit code and not by what it prints: it is
# a GUI program, so it has no console.
#Requires -Version 7
param(
  [Parameter(Mandatory = $true)][string]$Setup,
  [string]$Msi = ""
)
$ErrorActionPreference = "Stop"

function Say($text) { Write-Host ""; Write-Host "== $text" }
function Fail($text) {
  if ($env:GITHUB_ACTIONS) { Write-Host "::error::$text" }
  throw $text
}

# Where an installer may put the program: the user's own Programs folder
# (the .exe asks for the lowest privileges it can get with), and the
# machine's (the .msi is per-machine).
$places = @(
  (Join-Path $env:LOCALAPPDATA "Programs\Concat"),
  (Join-Path $env:ProgramFiles "Concat")
)
if (${env:ProgramFiles(x86)}) { $places += (Join-Path ${env:ProgramFiles(x86)} "Concat") }

function Find-Installed {
  foreach ($place in $places) {
    $exe = Join-Path $place "concat.exe"
    if (Test-Path $exe) { return $exe }
  }
  return $null
}

# Runs the installed program and fails unless it exits 0.
function Expect-Ok($what, $file) {
  $info = New-Object System.Diagnostics.ProcessStartInfo
  $info.FileName = $file
  [void]$info.ArgumentList.Add("--version")
  $info.UseShellExecute = $false
  $info.RedirectStandardOutput = $true
  $info.RedirectStandardError = $true
  # The program's own folder and nothing else: every DLL it loads is
  # beside it, and the runner's own copies are not to be found.
  $info.WorkingDirectory = [IO.Path]::GetTempPath()
  $process = [System.Diagnostics.Process]::Start($info)
  $stdout = $process.StandardOutput.ReadToEndAsync()
  $stderr = $process.StandardError.ReadToEndAsync()
  if (-not $process.WaitForExit(120000)) {
    $process.Kill($true)
    Fail "${what}: still running after 120 s"
  }
  $process.WaitForExit()
  $out = ($stdout.Result + $stderr.Result).Trim()
  if ($out) { $out -split "`n" | Select-Object -Last 20 | ForEach-Object { Write-Host "  $($_.TrimEnd())" } }
  if ($process.ExitCode -ne 0) {
    Fail ("${what}: exited with 0x{0:X8}" -f $process.ExitCode)
  }
  Write-Host "  exited 0"
}

# ── the .exe ───────────────────────────────────────────────────────────
$setup = (Resolve-Path $Setup).Path
Say "installing $setup"
$installed = Start-Process -FilePath $setup `
  -ArgumentList "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-" `
  -Wait -PassThru
if ($installed.ExitCode -ne 0) { Fail "the setup program exited with $($installed.ExitCode)" }

$exe = Find-Installed
if (-not $exe) { Fail "the setup program installed nothing: no concat.exe in $($places -join ', ')" }
Say "the setup program installed $exe"
Expect-Ok "the installed concat.exe" $exe

# And it takes it away again: an installer that cannot uninstall is an
# installer a person cannot get rid of.
$uninstaller = Get-ChildItem (Split-Path $exe) -Filter "unins*.exe" -ErrorAction SilentlyContinue |
  Select-Object -First 1
if (-not $uninstaller) { Fail "the setup program left no uninstaller beside the program" }
$removed = Start-Process -FilePath $uninstaller.FullName `
  -ArgumentList "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART" -Wait -PassThru
if ($removed.ExitCode -ne 0) { Fail "the uninstaller exited with $($removed.ExitCode)" }
if (Find-Installed) { Fail "the uninstaller left the program behind" }
Write-Host "  and the uninstaller took it away again"

# ── the .msi ───────────────────────────────────────────────────────────
if ($Msi) {
  $msi = (Resolve-Path $Msi).Path
  Say "installing $msi"
  $installed = Start-Process -FilePath "msiexec.exe" `
    -ArgumentList "/i", "`"$msi`"", "/qn", "/norestart" -Wait -PassThru
  # 0 is a clean install, 3010 is one that wants the machine restarted.
  if ($installed.ExitCode -ne 0 -and $installed.ExitCode -ne 3010) {
    Fail "msiexec exited with $($installed.ExitCode)"
  }
  $exe = Find-Installed
  if (-not $exe) { Fail "the .msi installed nothing: no concat.exe in $($places -join ', ')" }
  Say "the .msi installed $exe"
  Expect-Ok "the installed concat.exe" $exe

  Say "removing the .msi"
  $removed = Start-Process -FilePath "msiexec.exe" `
    -ArgumentList "/x", "`"$msi`"", "/qn", "/norestart" -Wait -PassThru
  if ($removed.ExitCode -ne 0 -and $removed.ExitCode -ne 3010) {
    Fail "msiexec /x exited with $($removed.ExitCode)"
  }
  if (Find-Installed) { Fail "the .msi left the program behind" }
  Write-Host "  and it took it away again"
}

Say "ok"
