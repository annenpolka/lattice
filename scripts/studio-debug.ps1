# Launch Lattice Studio without inheriting a closed pipe (Windows 0x800700e8).
# Builds the bin, starts it detached, waits, then prints the durable log.
#
#   ./scripts/studio-debug.ps1
#   ./scripts/studio-debug.ps1 examples/gameplay-commentary/main.vel
#   ./scripts/studio-debug.ps1 -NoPreview C:\path\to\main.vel
#
# Log file: $env:LATTICE_STUDIO_LOG or %LOCALAPPDATA%\lattice\studio.log
# Isolate GPUI (skip FFmpeg): -NoPreview  or  $env:LATTICE_STUDIO_PREVIEW=0

param(
    [string]$Vel,
    [switch]$NoPreview,
    [int]$WaitSeconds = 8,
    [switch]$Release
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if (-not $Vel) {
    $Vel = Join-Path $Root "examples\gameplay-commentary\main.vel"
}
$Vel = (Resolve-Path $Vel).Path

$profile = if ($Release) { "release" } else { "debug" }
$cargoArgs = @("build", "-p", "lattice-studio", "--features", "window")
if ($Release) { $cargoArgs += "--release" }
Write-Host "building lattice-studio ($profile)..."
& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed: $LASTEXITCODE"
}

$exe = Join-Path $Root "target\$profile\lattice-studio.exe"
if (-not (Test-Path $exe)) {
    throw "missing $exe"
}

$logDir = Join-Path $env:LOCALAPPDATA "lattice"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
$log = if ($env:LATTICE_STUDIO_LOG) { $env:LATTICE_STUDIO_LOG } else { Join-Path $logDir "studio.log" }
$stdout = Join-Path $logDir "studio.stdout.log"
$stderr = Join-Path $logDir "studio.stderr.log"
$env:LATTICE_STUDIO_LOG = $log
$env:RUST_BACKTRACE = "1"
if ($NoPreview) {
    $env:LATTICE_STUDIO_PREVIEW = "0"
}

Get-Process lattice-studio -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host "stopping existing lattice-studio pid=$($_.Id)"
    Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
}
Remove-Item $stdout, $stderr -ErrorAction SilentlyContinue

"==== studio-debug $(Get-Date -Format o) vel=$Vel preview=$(if ($NoPreview) {'off'} else {'on'}) ====" |
    Add-Content -Path $log -Encoding utf8

Write-Host "starting $exe"
Write-Host "  vel  $Vel"
Write-Host "  log  $log"

$proc = Start-Process -FilePath $exe -ArgumentList @($Vel) -WorkingDirectory $Root -PassThru `
    -RedirectStandardOutput $stdout -RedirectStandardError $stderr
$pidStudio = $proc.Id
Write-Host "pid $pidStudio — waiting ${WaitSeconds}s"

# Native abort never hits the Rust panic hook. A sibling waiter records the Win32 exit code.
$watch = @"
`$id = $pidStudio
`$log = '$($log.Replace("'", "''"))'
try {
    `$p = Get-Process -Id `$id -ErrorAction Stop
    `$p.WaitForExit()
    `$code = `$p.ExitCode
    `$hex = '{0} (0x{1:X8})' -f `$code, [uint32](`$code -band 0xFFFFFFFF)
} catch {
    `$hex = "unknown (`$(`$_.Exception.Message))"
}
Add-Content -LiteralPath `$log -Encoding utf8 -Value ("EXIT pid=`$id code=`$hex at `$(Get-Date -Format o)")
"@
Start-Process -FilePath "pwsh" -WindowStyle Hidden -ArgumentList @("-NoProfile", "-Command", $watch) | Out-Null

Start-Sleep -Seconds $WaitSeconds
$proc.Refresh()
$alive = -not $proc.HasExited

function Show-Tail([string]$path, [string]$title) {
    Write-Host ""
    Write-Host "----- $title ($path) -----"
    if (Test-Path $path) {
        Get-Content $path -Tail 40
    } else {
        Write-Host "(missing)"
    }
}

Show-Tail $log "studio.log"
Show-Tail $stdout "stdout (GPUI leftovers)"
Show-Tail $stderr "stderr (GPUI leftovers)"

if ($alive) {
    Write-Host ""
    Write-Host "STILL RUNNING pid=$pidStudio"
    Write-Host "stop: Stop-Process -Id $pidStudio"
    exit 0
}

$code = $proc.ExitCode
Write-Host ""
Write-Host "EXITED pid=$pidStudio code=$code after ${WaitSeconds}s"
Write-Host "This is the crash to debug. File log is the source of truth (not the console)."
exit 1
