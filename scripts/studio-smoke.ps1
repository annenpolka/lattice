# Process smoke for Lattice Studio. Not CHI-54 (no VLM / window robot).
#
# Builds the bins, resolves an Alpha-shaped A/V fixture, starts Studio detached
# with autoplay + timed quit, then asserts the durable log saw a window,
# an explicit renderer selection, and several in-memory preview frames.
# Fails on PANIC, native abort, or a hang past the smoke timeout.
#
#   ./scripts/studio-smoke.ps1
#   ./scripts/studio-smoke.ps1 examples/warframe-cut/main.vel
#   ./scripts/studio-smoke.ps1 -Renderer gpu-dx12
#   ./scripts/studio-smoke.ps1 -Renderer gpu-dx12 -Adapter "NVIDIA"
#   ./scripts/studio-smoke.ps1 -Renderer gpu-dx12 -SoakMinutes 30
#   ./scripts/studio-smoke.ps1 -NoPreview
#
# GitHub-hosted Windows is not a reliable interactive GPU/DWM session.
# Keep this as a local / desktop gate. cargo test --workspace stays the CI gate.

param(
    [string]$Vel,
    [int]$SmokeMs = 15000,
    [int]$WaitSeconds = 0,
    [ValidateSet("cpu", "gpu-dx12")]
    [string]$Renderer = "cpu",
    [string]$Adapter,
    [ValidateRange(0, 1440)]
    [int]$SoakMinutes = 0,
    [ValidateRange(16, 16384)]
    [int]$MaxWorkingSetGrowthMB = 512,
    [ValidateRange(16, 16384)]
    [int]$MaxPrivateMemoryGrowthMB = 512,
    [ValidateRange(16, 16384)]
    [int]$MaxDedicatedGpuGrowthMB = 512,
    [ValidateRange(5, 600)]
    [int]$SoakWarmupSeconds = 30,
    [switch]$Release,
    [switch]$NoPreview
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if ($Adapter -and $Renderer -ne "gpu-dx12") {
    throw "-Adapter is only valid with -Renderer gpu-dx12"
}
if ($NoPreview -and $Renderer -eq "gpu-dx12") {
    throw "-Renderer gpu-dx12 requires preview; remove -NoPreview"
}
if ($SoakMinutes -gt 0 -and $SoakWarmupSeconds -ge ($SoakMinutes * 60)) {
    throw "-SoakWarmupSeconds must be shorter than the requested soak"
}

function Fail([string]$message) {
    Write-Host ""
    Write-Host "SMOKE FAIL: $message"
    exit 1
}

function Show-Tail([string]$path, [string]$title) {
    Write-Host ""
    Write-Host "----- $title ($path) -----"
    if (Test-Path $path) {
        Get-Content $path -Tail 60
    } else {
        Write-Host "(missing)"
    }
}

function Get-DedicatedGpuBytes([int]$processId) {
    try {
        $samples = (Get-Counter '\GPU Process Memory(*)\Dedicated Usage' -ErrorAction Stop).CounterSamples
        $matches = @($samples | Where-Object { $_.InstanceName -match "^pid_$processId(_|$)" })
        if ($matches.Count -eq 0) {
            return $null
        }
        return [int64](($matches | Measure-Object -Property CookedValue -Sum).Sum)
    } catch {
        return $null
    }
}

function Format-MiB([int64]$bytes) {
    return [Math]::Round($bytes / 1MB, 1)
}

function New-SmokeProject(
    [int]$DurationSeconds = 7,
    [switch]$VideoOnly
) {
    if ($DurationSeconds -lt 7) {
        Fail "smoke fixture duration must be at least 7 seconds"
    }
    $dir = Join-Path $env:TEMP ("lattice-studio-smoke-project-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $mp4 = Join-Path $dir "capture.mp4"
    $velPath = Join-Path $dir "main.vel"
    if ($VideoOnly) {
        & ffmpeg -y -hide_banner -loglevel error `
            -f lavfi -i "testsrc=size=320x180:rate=10:duration=$DurationSeconds" `
            -an -pix_fmt yuv420p $mp4
    } else {
        & ffmpeg -y -hide_banner -loglevel error `
            -f lavfi -i "testsrc=size=320x180:rate=10:duration=$DurationSeconds" `
            -f lavfi -i "sine=frequency=440:sample_rate=44100:duration=$DurationSeconds" `
            -shortest -pix_fmt yuv420p $mp4
    }
    if ($LASTEXITCODE -ne 0) {
        Fail "ffmpeg fixture failed: $LASTEXITCODE"
    }
    $probeText = & ffprobe -v error -show_entries format=duration `
        -show_entries stream=codec_type -of json $mp4
    if ($LASTEXITCODE -ne 0) {
        Fail "ffprobe fixture verification failed: $LASTEXITCODE"
    }
    try {
        $probe = $probeText | ConvertFrom-Json
    } catch {
        Fail "ffprobe fixture verification did not return JSON: $probeText"
    }
    $actualDuration = [double]::Parse(
        [string]$probe.format.duration,
        [Globalization.CultureInfo]::InvariantCulture
    )
    if ($actualDuration -lt ($DurationSeconds - 0.25)) {
        Fail "fixture duration was ${actualDuration}s; expected about ${DurationSeconds}s"
    }
    $videoStreams = @($probe.streams | Where-Object codec_type -eq "video")
    $audioStreams = @($probe.streams | Where-Object codec_type -eq "audio")
    if ($videoStreams.Count -ne 1) {
        Fail "fixture must contain exactly one video stream"
    }
    $expectedAudioStreams = if ($VideoOnly) { 0 } else { 1 }
    if ($audioStreams.Count -ne $expectedAudioStreams) {
        Fail "fixture audio stream count was $($audioStreams.Count); expected $expectedAudioStreams"
    }
    $clipEndSeconds = $DurationSeconds - 1
    $audioDirectives = if ($VideoOnly) { "" } else {
        @"
  gain clip by -3
  speech "Resolved smoke" { at 2s for 1s }
"@
    }
    $velText = @"
project "smoke"

convention commentary

media game "capture.mp4"

scene demo {
  game[0s..${clipEndSeconds}s] as clip
  freeze clip at 2s for 1s
  fade clip { at 0s for 0.5s }
  title "Smoke" { at 1s for 3s }
  callout "Hold" { at 2s for 1s }
$audioDirectives
}
"@
    $utf8 = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText($velPath, $velText, $utf8)
    return $velPath
}

foreach ($tool in @("ffmpeg", "ffprobe")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        Fail "$tool is not on PATH"
    }
}

if (-not $Vel) {
    $fixtureDurationSeconds = if ($SoakMinutes -gt 0) {
        ($SoakMinutes * 60) + 10
    } else {
        7
    }
    $Vel = New-SmokeProject -DurationSeconds $fixtureDurationSeconds -VideoOnly:($SoakMinutes -gt 0)
}
$Vel = (Resolve-Path $Vel).Path

if ($SoakMinutes -gt 0) {
    $SmokeMs64 = [int64]$SoakMinutes * 60 * 1000
    if ($SmokeMs64 -gt [int]::MaxValue) {
        Fail "SoakMinutes is too large for LATTICE_STUDIO_SMOKE_MS"
    }
    $SmokeMs = [int]$SmokeMs64
}

$profile = if ($Release) { "release" } else { "debug" }
$targetRoot = if ($env:CARGO_TARGET_DIR) {
    if ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
        $env:CARGO_TARGET_DIR
    } else {
        Join-Path $Root $env:CARGO_TARGET_DIR
    }
} else {
    Join-Path $Root "target"
}
$cargoArgs = @("build", "-p", "lattice-studio", "--features", "window")
if ($Release) { $cargoArgs += "--release" }
Write-Host "building lattice-studio ($profile)..."
& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed: $LASTEXITCODE"
}

$cliBuildArgs = @("build", "-p", "lattice-cli")
if ($Release) { $cliBuildArgs += "--release" }
Write-Host "building lattice CLI ($profile)..."
& cargo @cliBuildArgs
if ($LASTEXITCODE -ne 0) {
    throw "cargo CLI build failed: $LASTEXITCODE"
}

$exe = Join-Path $targetRoot "$profile\lattice-studio.exe"
if (-not (Test-Path $exe)) {
    Fail "missing $exe"
}


$cli = Join-Path $targetRoot "$profile\lattice.exe"
if (-not (Test-Path $cli)) {
    Fail "missing $cli"
}
Write-Host "resolving generated media..."
& $cli --json resolve $Vel
if ($LASTEXITCODE -ne 0) {
    Fail "lattice resolve failed: $LASTEXITCODE"
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$logDir = Join-Path $env:TEMP "lattice-studio-smoke"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
$log = Join-Path $logDir "studio-smoke-$stamp.log"
$stdout = Join-Path $logDir "studio-smoke-$stamp.stdout.log"
$stderr = Join-Path $logDir "studio-smoke-$stamp.stderr.log"
$telemetry = Join-Path $logDir "studio-smoke-$stamp.telemetry.csv"

$env:LATTICE_STUDIO_LOG = $log
$env:LATTICE_STUDIO_AUTOPLAY = "1"
$env:LATTICE_STUDIO_SMOKE_MS = "$SmokeMs"
$env:LATTICE_STUDIO_RENDERER = $Renderer
if ($SoakMinutes -gt 0) {
    $env:LATTICE_STUDIO_AUDIO_MONITOR = "0"
} else {
    Remove-Item Env:LATTICE_STUDIO_AUDIO_MONITOR -ErrorAction SilentlyContinue
}
if ($Adapter) {
    $env:LATTICE_DX12_ADAPTER = $Adapter
} else {
    Remove-Item Env:LATTICE_DX12_ADAPTER -ErrorAction SilentlyContinue
}
$env:RUST_BACKTRACE = "1"
if ($NoPreview) {
    $env:LATTICE_STUDIO_PREVIEW = "0"
} else {
    Remove-Item Env:LATTICE_STUDIO_PREVIEW -ErrorAction SilentlyContinue
}

"==== studio-smoke $(Get-Date -Format o) vel=$Vel smokeMs=$SmokeMs preview=$(if ($NoPreview) {'off'} else {'on'}) audio=$(if ($SoakMinutes -gt 0) {'off-soak'} else {'on'}) renderer=$Renderer adapter=$(if ($Adapter) {$Adapter} else {'auto-high-performance'}) ====" |
    Add-Content -Path $log -Encoding utf8

Write-Host "starting $exe"
Write-Host "  vel   $Vel"
Write-Host "  log   $log"
Write-Host "  smoke ${SmokeMs}ms autoplay=on renderer=$Renderer adapter=$(if ($Adapter) {$Adapter} else {'auto-high-performance'})"
if ($SoakMinutes -gt 0) {
    Write-Host "  soak  ${SoakMinutes}m; WS + private + dedicated GPU telemetry -> $telemetry"
}

$proc = Start-Process -FilePath $exe -ArgumentList @($Vel) -WorkingDirectory $Root -PassThru `
    -RedirectStandardOutput $stdout -RedirectStandardError $stderr -WindowStyle Hidden
$pidStudio = $proc.Id
Write-Host "pid $pidStudio"

if ($WaitSeconds -le 0) {
    $WaitSeconds = [Math]::Ceiling($SmokeMs / 1000.0) + 15
}

$samples = [System.Collections.Generic.List[object]]::new()
$deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
while (-not $proc.HasExited -and [DateTime]::UtcNow -lt $deadline) {
    try {
        $proc.Refresh()
        $dedicatedGpu = if ($Renderer -eq "gpu-dx12") {
            Get-DedicatedGpuBytes $pidStudio
        } else {
            $null
        }
        $samples.Add([pscustomobject]@{
            elapsed_seconds = [Math]::Round(((Get-Date) - $proc.StartTime).TotalSeconds, 1)
            working_set_bytes = [int64]$proc.WorkingSet64
            private_bytes = [int64]$proc.PrivateMemorySize64
            dedicated_gpu_bytes = $dedicatedGpu
        })
    } catch {
        if (-not $proc.HasExited) {
            Write-Warning "telemetry sample failed: $($_.Exception.Message)"
        }
    }
    if (-not $proc.HasExited) {
        Start-Sleep -Milliseconds 1000
        $proc.Refresh()
    }
}
if (-not $proc.HasExited) {
    Stop-Process -Id $pidStudio -Force -ErrorAction SilentlyContinue
    if ($samples.Count -gt 0) {
        $samples | Export-Csv -NoTypeInformation -Encoding utf8 -Path $telemetry
    }
    Show-Tail $log "studio-smoke.log"
    Show-Tail $stdout "stdout"
    Show-Tail $stderr "stderr"
    Fail "still running after ${WaitSeconds}s (smoke did not quit) pid=$pidStudio"
}

if ($samples.Count -gt 0) {
    $samples | Export-Csv -NoTypeInformation -Encoding utf8 -Path $telemetry
}

$proc.Refresh()
$code = $proc.ExitCode
$logText = if (Test-Path $log) { Get-Content $log -Raw } else { "" }
$stdoutText = if (Test-Path $stdout) { Get-Content $stdout -Raw } else { "" }
$stderrText = if (Test-Path $stderr) { Get-Content $stderr -Raw } else { "" }
$processText = "$logText`n$stdoutText`n$stderrText"

Show-Tail $log "studio-smoke.log"
Show-Tail $stdout "stdout (GPUI leftovers)"
Show-Tail $stderr "stderr (GPUI leftovers)"

if ($code -ne 0) {
    $exitBits = [BitConverter]::ToUInt32([BitConverter]::GetBytes([int32]$code), 0)
    $hex = '{0:X8}' -f $exitBits
    Fail "exit code $code (0x$hex) pid=$pidStudio"
}
if ($processText -match "PANIC|panicked at|fatal runtime error") {
    Fail "Studio log/stdout/stderr contains a panic or fatal runtime error"
}
if ($processText -match "preview worker panic") {
    Fail "preview worker panicked"
}
if ($logText -notmatch "open_window ok") {
    Fail "missing open_window ok"
}
if ($logText -notmatch "play samples") {
    Fail "missing play samples (autoplay did not start the session clock)"
}
if ($logText -match "audio blocks A/V play") {
    Fail "AudioPlan/device failure blocked synchronized playback"
}
if ($SoakMinutes -eq 0) {
    if ($logText -notmatch "audio ready frames=[0-9]+ windows=[0-9]+") {
        Fail "missing prepared AudioPlan PCM"
    }
    if ($logText -notmatch "audio sync reason=play transport=Started") {
        Fail "audio stream did not start from the shared playhead clock"
    }
    $audioReadyIndex = $logText.IndexOf("audio ready frames=")
    $playIndex = $logText.IndexOf("play samples AudioPlan+video")
    if ($audioReadyIndex -lt 0 -or $playIndex -lt 0 -or $playIndex -lt $audioReadyIndex) {
        Fail "video playback started before AudioPlan PCM was ready"
    }
} elseif ($logText -notmatch "audio monitor explicitly disabled") {
    Fail "renderer soak did not explicitly disable the AudioPlan monitor"
}
if (-not $NoPreview -and $logText -notmatch "preview frame") {
    Fail "missing preview frame (still pipeline did not publish)"
}
if ($Renderer -eq "cpu") {
    $rendererPattern = "preview renderer requested=require_cpu, active=cpu"
} else {
    $rendererPattern = "preview renderer requested=require_gpu_dx12, active=gpu_dx12"
}
if (-not $NoPreview -and $logText -notmatch $rendererPattern) {
    Fail "missing explicit $Renderer renderer selection"
}
if (-not $NoPreview -and $Adapter) {
    $escapedAdapter = [regex]::Escape($Adapter)
    if ($logText -notmatch "preview renderer .*adapter=[^,]*$escapedAdapter") {
        Fail "selected renderer did not report adapter matching '$Adapter'"
    }
}
if ($Renderer -eq "gpu-dx12" -and $logText -match "preview renderer recreate") {
    Fail "GPU runtime was recreated during playback"
}
if (-not $NoPreview -and $logText -notmatch "preview frame .* memory [0-9]+x[0-9]+") {
    Fail "preview did not publish an in-memory RawFrame"
}
if (-not $NoPreview) {
    $previewTimes = [regex]::Matches($logText, "preview frame ([^ ]+) memory") |
        ForEach-Object { $_.Groups[1].Value } |
        Sort-Object -Unique
    if (@($previewTimes).Count -lt 3) {
        Fail "expected at least 3 distinct preview times, got $(@($previewTimes).Count)"
    }
    if ($SoakMinutes -gt 0) {
        $previewSeconds = [regex]::Matches($logText, "preview frame ([0-9]+(?:\.[0-9]+)?)s memory") |
            ForEach-Object { [double]::Parse($_.Groups[1].Value, [Globalization.CultureInfo]::InvariantCulture) }
        $maxPreviewSeconds = ($previewSeconds | Measure-Object -Maximum).Maximum
        $minimumPreviewSeconds = ($SoakMinutes * 60) - 5
        if ($null -eq $maxPreviewSeconds -or $maxPreviewSeconds -lt $minimumPreviewSeconds) {
            Fail "playback ended early at ${maxPreviewSeconds}s; expected at least ${minimumPreviewSeconds}s"
        }
    }
    $cache = Join-Path (Split-Path -Parent $Vel) ".lattice-cache"
    $diskFrames = @(Get-ChildItem -LiteralPath $cache -Filter "frame-*.png" -File -ErrorAction SilentlyContinue)
    if ($diskFrames.Count -ne 0) {
        Fail "Play hot path wrote $($diskFrames.Count) PNG frame(s) to $cache"
    }
}
if ($logText -notmatch "smoke quit") {
    Fail "missing smoke quit (process exited without the watchdog)"
}

if ($samples.Count -gt 0) {
    $measuredSamples = if ($SoakMinutes -gt 0) {
        @($samples | Where-Object { $_.elapsed_seconds -ge $SoakWarmupSeconds })
    } else {
        @($samples)
    }
    if ($measuredSamples.Count -eq 0) {
        Fail "no telemetry samples remained after ${SoakWarmupSeconds}s warm-up"
    }
    $first = $measuredSamples[0]
    $maxWs = ($measuredSamples | Measure-Object -Property working_set_bytes -Maximum).Maximum
    $maxPrivate = ($measuredSamples | Measure-Object -Property private_bytes -Maximum).Maximum
    $wsGrowth = [int64]$maxWs - [int64]$first.working_set_bytes
    $privateGrowth = [int64]$maxPrivate - [int64]$first.private_bytes
    $gpuSamples = @($measuredSamples | Where-Object { $null -ne $_.dedicated_gpu_bytes })
    $gpuGrowth = $null
    if ($gpuSamples.Count -gt 0) {
        $maxGpu = ($gpuSamples | Measure-Object -Property dedicated_gpu_bytes -Maximum).Maximum
        $gpuGrowth = [int64]$maxGpu - [int64]$gpuSamples[0].dedicated_gpu_bytes
    }
    Write-Host "telemetry samples=$($measuredSamples.Count) warmup=${SoakWarmupSeconds}s wsGrowth=$(Format-MiB $wsGrowth)MiB privateGrowth=$(Format-MiB $privateGrowth)MiB gpuGrowth=$(if ($null -eq $gpuGrowth) {'n/a'} else {"$(Format-MiB $gpuGrowth)MiB"})"
    if ($SoakMinutes -gt 0) {
        if ($wsGrowth -gt ([int64]$MaxWorkingSetGrowthMB * 1MB)) {
            Fail "working set grew $(Format-MiB $wsGrowth)MiB (limit ${MaxWorkingSetGrowthMB}MiB)"
        }
        if ($privateGrowth -gt ([int64]$MaxPrivateMemoryGrowthMB * 1MB)) {
            Fail "private memory grew $(Format-MiB $privateGrowth)MiB (limit ${MaxPrivateMemoryGrowthMB}MiB)"
        }
        if ($Renderer -eq "gpu-dx12" -and $null -eq $gpuGrowth) {
            Fail "dedicated GPU process-memory counter was unavailable during DX12 soak"
        }
        if ($null -ne $gpuGrowth -and $gpuGrowth -gt ([int64]$MaxDedicatedGpuGrowthMB * 1MB)) {
            Fail "dedicated GPU memory grew $(Format-MiB $gpuGrowth)MiB (limit ${MaxDedicatedGpuGrowthMB}MiB)"
        }
    }
}

Write-Host ""
Write-Host "SMOKE OK pid=$pidStudio code=$code"
exit 0
