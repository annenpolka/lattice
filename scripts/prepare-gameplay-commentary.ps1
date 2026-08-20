# Generate the explicit local media fixture used by the Alpha gameplay example.
# Production render never calls this script or substitutes missing media.

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$outputPath = Join-Path $repoRoot "examples\gameplay-commentary\capture.mp4"

$ffmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue
if (-not $ffmpeg) {
    throw "ffmpeg is not on PATH"
}

$ffprobe = Get-Command ffprobe -ErrorAction SilentlyContinue
if (-not $ffprobe) {
    throw "ffprobe is not on PATH"
}

Write-Host "Generating $outputPath"
& $ffmpeg.Source -y -hide_banner -loglevel error `
    -f lavfi -i "testsrc=duration=21:size=320x180:rate=10" `
    -f lavfi -i "sine=frequency=440:sample_rate=44100:duration=21" `
    -shortest -pix_fmt yuv420p $outputPath
if ($LASTEXITCODE -ne 0) {
    throw "ffmpeg fixture generation failed with exit code $LASTEXITCODE"
}

if (-not (Test-Path -LiteralPath $outputPath -PathType Leaf)) {
    throw "ffmpeg did not create $outputPath"
}

$probeJson = & $ffprobe.Source -v error `
    -show_entries "format=duration:stream=codec_type,width,height,r_frame_rate" `
    -of json $outputPath
if ($LASTEXITCODE -ne 0) {
    throw "ffprobe fixture verification failed with exit code $LASTEXITCODE"
}

$probe = $probeJson | ConvertFrom-Json
$video = @($probe.streams) | Where-Object { $_.codec_type -eq "video" } | Select-Object -First 1
$audio = @($probe.streams) | Where-Object { $_.codec_type -eq "audio" } | Select-Object -First 1
$duration = [double]::Parse(
    [string]$probe.format.duration,
    [System.Globalization.CultureInfo]::InvariantCulture
)

if (-not $video -or $video.width -ne 320 -or $video.height -ne 180) {
    throw "fixture verification failed: expected a 320x180 video stream"
}
if ($video.r_frame_rate -ne "10/1") {
    throw "fixture verification failed: expected 10 fps, got $($video.r_frame_rate)"
}
if (-not $audio) {
    throw "fixture verification failed: expected an audio stream"
}
if ($duration -lt 20.9 -or $duration -gt 21.1) {
    throw "fixture verification failed: expected about 21 seconds, got $duration"
}

Write-Host "Fixture OK: 21s, 320x180, 10fps, video + 440Hz audio"
Write-Output $outputPath
