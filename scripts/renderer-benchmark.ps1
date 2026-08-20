# Local Windows DX12/CPU export benchmark for CHI-58.
# Requires PowerShell 7, FFmpeg/ffprobe, and a DX12 adapter.
#
#   ./scripts/renderer-benchmark.ps1
#   ./scripts/renderer-benchmark.ps1 examples/gameplay-commentary/main.vel -Width 1920 -Height 1080 -Fps 30
#   ./scripts/renderer-benchmark.ps1 -Adapter "NVIDIA"

param(
    [string]$Vel = "examples/gameplay-commentary/main.vel",
    [ValidateRange(2, 16384)]
    [int]$Width = 1920,
    [ValidateRange(2, 16384)]
    [int]$Height = 1080,
    [ValidateRange(1, 240)]
    [int]$Fps = 30,
    [ValidateRange(1, 10)]
    [int]$Iterations = 1,
    [string]$Adapter,
    [switch]$DebugBuild
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if ($Adapter) {
    $env:LATTICE_DX12_ADAPTER = $Adapter
} else {
    Remove-Item Env:LATTICE_DX12_ADAPTER -ErrorAction SilentlyContinue
}

function Fail([string]$message) {
    Write-Host "BENCH FAIL: $message"
    exit 1
}

function Invoke-Render(
    [string]$cli,
    [string]$velPath,
    [string]$renderer,
    [string]$output,
    [int]$width,
    [int]$height,
    [int]$fps,
    [string]$adapterFilter
) {
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $jsonText = & $cli --json render $velPath -o $output `
        --renderer $renderer --width $width --height $height --fps $fps
    $exitCode = $LASTEXITCODE
    $watch.Stop()
    if ($exitCode -ne 0) {
        Fail "$renderer render failed with exit code ${exitCode}: $($jsonText -join [Environment]::NewLine)"
    }
    try {
        $payload = $jsonText | ConvertFrom-Json
    } catch {
        Fail "$renderer render did not return JSON: $jsonText"
    }
    if (-not $payload.ok) {
        Fail "$renderer render returned ok=false"
    }
    $active = $payload.renderer.active
    $expected = if ($renderer -eq "cpu") { "cpu" } else { "gpu_dx12" }
    $expectedRequest = if ($renderer -eq "cpu") { "require_cpu" } else { "require_gpu_dx12" }
    if ($payload.renderer.requested -ne $expectedRequest) {
        Fail "$renderer render reported requested='$($payload.renderer.requested)'"
    }
    if ($active -ne $expected) {
        Fail "$renderer requested but active renderer was '$active'"
    }
    $activeAdapter = $payload.renderer.adapter
    if ($renderer -eq "cpu" -and $null -ne $activeAdapter) {
        Fail "CPU render unexpectedly reported adapter '$activeAdapter'"
    }
    if ($renderer -eq "gpu-dx12") {
        if ([string]::IsNullOrWhiteSpace($activeAdapter)) {
            Fail "GPU render did not report renderer.adapter"
        }
        if ($adapterFilter -and $activeAdapter.IndexOf(
            $adapterFilter,
            [System.StringComparison]::OrdinalIgnoreCase
        ) -lt 0) {
            Fail "selected adapter '$activeAdapter' did not match -Adapter '$adapterFilter'"
        }
    }
    if (
        $payload.spec.width -ne $width -or
        $payload.spec.height -ne $height -or
        $payload.spec.fps_num -ne $fps -or
        $payload.spec.fps_den -ne 1
    ) {
        Fail "JSON output spec mismatch: $($payload.spec | ConvertTo-Json -Compress)"
    }
    return [pscustomobject]@{
        requested = $renderer
        active = $active
        adapter = $payload.renderer.adapter
        reason = $payload.renderer.reason
        seconds = [Math]::Round($watch.Elapsed.TotalSeconds, 3)
        output = $output
    }
}

if ($Width % 2 -ne 0 -or $Height % 2 -ne 0) {
    Fail "Width and Height must be even for yuv420p output"
}
foreach ($tool in @("ffmpeg", "ffprobe")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        Fail "$tool is not on PATH"
    }
}
if ($Vel -eq "examples/gameplay-commentary/main.vel" -and -not (Test-Path "examples/gameplay-commentary/capture.mp4")) {
    & "$PSScriptRoot/prepare-gameplay-commentary.ps1"
    if ($LASTEXITCODE -ne 0) {
        Fail "fixture preparation failed"
    }
}
$Vel = (Resolve-Path $Vel).Path

$profile = if ($DebugBuild) { "debug" } else { "release" }
$cargoArgs = @("build", "-p", "lattice-cli")
if (-not $DebugBuild) { $cargoArgs += "--release" }
& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) {
    Fail "cargo build failed"
}
$targetRoot = if ($env:CARGO_TARGET_DIR) {
    if ([IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) { $env:CARGO_TARGET_DIR } else { Join-Path $Root $env:CARGO_TARGET_DIR }
} else {
    Join-Path $Root "target"
}
$cli = Join-Path $targetRoot "$profile/lattice.exe"
if (-not (Test-Path $cli)) {
    Fail "missing $cli"
}

& $cli --json resolve $Vel | Out-Null
if ($LASTEXITCODE -ne 0) {
    Fail "resolve failed"
}

$runDir = Join-Path $env:TEMP ("lattice-renderer-benchmark-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$rows = [System.Collections.Generic.List[object]]::new()
foreach ($iteration in 1..$Iterations) {
    foreach ($renderer in @("cpu", "gpu-dx12")) {
        $output = Join-Path $runDir "$renderer-$iteration.mp4"
        Write-Host "benchmark renderer=$renderer iteration=$iteration spec=${Width}x${Height}@${Fps}"
        $row = Invoke-Render $cli $Vel $renderer $output $Width $Height $Fps $Adapter
        $probeText = & ffprobe -v error -select_streams v:0 `
            -show_entries stream=width,height,avg_frame_rate -of json $output
        if ($LASTEXITCODE -ne 0) {
            Fail "ffprobe failed for ${renderer} output with exit code $LASTEXITCODE"
        }
        try {
            $probe = $probeText | ConvertFrom-Json
        } catch {
            Fail "ffprobe did not return JSON for ${renderer}: $($probeText -join [Environment]::NewLine)"
        }
        $streams = @($probe.streams)
        if ($streams.Count -ne 1) {
            Fail "ffprobe returned $($streams.Count) selected video streams for ${renderer}"
        }
        $stream = $streams[0]
        if ($stream.width -ne $Width -or $stream.height -ne $Height -or $stream.avg_frame_rate -ne "$Fps/1") {
            Fail "ffprobe mismatch for ${renderer}: $($stream.width)x$($stream.height)@$($stream.avg_frame_rate)"
        }
        $rows.Add([pscustomobject]@{
            iteration = $iteration
            requested = $row.requested
            active = $row.active
            adapter = $row.adapter
            reason = $row.reason
            width = $Width
            height = $Height
            fps = $Fps
            seconds = $row.seconds
            output = $row.output
        })
    }
}

$cpuAverage = ($rows | Where-Object requested -eq "cpu" | Measure-Object seconds -Average).Average
$gpuAverage = ($rows | Where-Object requested -eq "gpu-dx12" | Measure-Object seconds -Average).Average
$summary = [pscustomobject]@{
    vel = $Vel
    requested_adapter = if ($Adapter) { $Adapter } else { "auto-high-performance" }
    iterations = $Iterations
    spec = "${Width}x${Height}@${Fps}"
    cpu_seconds_average = [Math]::Round($cpuAverage, 3)
    gpu_seconds_average = [Math]::Round($gpuAverage, 3)
    gpu_over_cpu_speedup = [Math]::Round($cpuAverage / $gpuAverage, 3)
    runs = $rows
}
$result = Join-Path $runDir "result.json"
$summary | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8NoBOM -Path $result
$summary | ConvertTo-Json -Depth 6
Write-Host "BENCH OK result=$result"
