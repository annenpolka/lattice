# Renderer dogfood (CHI-58)

This is the reproducible Windows DX12 evidence procedure for CHI-58. It is an execution checklist and result template, not a claim that the hardware matrix has passed.

## Prerequisites

- Windows 11 x64 desktop with PowerShell 7 (`pwsh`).
- Rust 1.97.1 and the repository dependencies.
- `ffmpeg` and `ffprobe` on `PATH`.
- A usable Windows default audio output device for Studio smoke.
- A DX12 adapter and current vendor driver for GPU runs.

Generate the checked-in example's ignored A/V fixture before using it directly:

```powershell
./scripts/prepare-gameplay-commentary.ps1
```

With no VEL path, `studio-smoke.ps1` instead creates and resolves a temporary A/V project.

## Adapter selection

`LATTICE_DX12_ADAPTER` is a case-insensitive substring filter over DX12 adapter names. If it is unset, Lattice requests the high-performance adapter. Both dogfood scripts accept `-Adapter` and set the filter for the child process:

```powershell
./scripts/studio-smoke.ps1 -Renderer gpu-dx12 -Adapter "NVIDIA"
./scripts/renderer-benchmark.ps1 -Adapter "NVIDIA"
```

The successful JSON renderer report must contain `requested=require_gpu_dx12`, `active=gpu_dx12`, and the full selected adapter name. A requested GPU never falls back to CPU; initialization/render failure exits nonzero with a typed failure kind.

## Reproducible runs

Strict CPU/DX12 pixel conformance is an explicit hardware gate. Ordinary workspace CI does not opportunistically run it against an arbitrary GitHub-hosted adapter:

```powershell
$env:LATTICE_REQUIRE_DX12_TESTS = "1"
$env:LATTICE_DX12_ADAPTER = "NVIDIA"
cargo test -p lattice-media gpu::tests::dx12_ --offline -- --nocapture
```

Change the adapter filter to `AMD` (or another vendor substring) to record each hardware row. When the gate is enabled, a missing/mismatched adapter or any initialization/render/conformance failure fails the test; there is no CPU fallback.

Short debug Studio smoke:

```powershell
./scripts/studio-smoke.ps1 -Renderer gpu-dx12 -Adapter "NVIDIA"
```

Debug 1080p CPU/GPU export comparison:

```powershell
./scripts/renderer-benchmark.ps1 -DebugBuild -Adapter "NVIDIA" -Width 1920 -Height 1080 -Fps 30 -Iterations 1
```

Release Studio smoke and export comparison:

```powershell
./scripts/studio-smoke.ps1 -Release -Renderer gpu-dx12 -Adapter "NVIDIA"
./scripts/renderer-benchmark.ps1 -Adapter "NVIDIA" -Width 1920 -Height 1080 -Fps 30 -Iterations 3
```

Thirty-minute release soak with the explicit default growth limits:

```powershell
./scripts/studio-smoke.ps1 -Release -Renderer gpu-dx12 -Adapter "NVIDIA" -SoakMinutes 30 `
  -SoakWarmupSeconds 30 `
  -MaxWorkingSetGrowthMB 512 `
  -MaxPrivateMemoryGrowthMB 512 `
  -MaxDedicatedGpuGrowthMB 512
```

Soak mode creates a video timeline ten seconds longer than the requested run and fails unless preview timestamps reach at least five seconds before the deadline. It sets `LATTICE_STUDIO_AUDIO_MONITOR=0` explicitly so this renderer-lifetime gate does not predecode thirty minutes of PCM. Run the short Studio smoke separately for AudioPlan/device synchronization.

For a CPU memory baseline, use `-Renderer cpu` and omit `-Adapter`; dedicated GPU telemetry is then not required.

## Thirty-minute thresholds

Growth is the maximum value after warm-up minus the first sample after warm-up. The CSV retains the raw one-second samples.

| Signal | Default gate | Evidence |
|---|---:|---|
| Process working set | ≤ 512 MiB growth | `working_set_bytes` |
| Process private memory | ≤ 512 MiB growth | `private_bytes` |
| Dedicated per-process GPU memory | ≤ 512 MiB growth | `dedicated_gpu_bytes` |

The DX12 soak fails if the Windows dedicated GPU process-memory counter is unavailable. Studio logs and telemetry are written under `%TEMP%\lattice-studio-smoke`; the benchmark prints the temporary `result.json` path. Preserve the log, CSV, benchmark JSON, commit SHA, Windows build, adapter name, and driver version with each result.

## Local results — 2026-08-20

These runs used the current working tree based on commit `9371455fec340e3c32da52711460cdbf1e33d3f1` on Windows 11. The result files remain under `%TEMP%`; copy them to durable CI or release storage before relying on them as long-term evidence.

| Adapter | Driver | Debug Studio smoke | Release 1080p30 export |
|---|---|---|---|
| NVIDIA GeForce RTX 5070 Ti | `32.0.16.1047` | PASS — A/V synchronized, 0.1 s in-memory frames, no runtime recreation / PNG / panic | CPU 15.026 s, DX12 3.316 s, **4.531×** speedup |
| AMD Radeon(TM) Graphics | `32.0.21041.1000` | PASS — A/V synchronized, 0.1 s in-memory frames, no runtime recreation / PNG / panic | CPU 15.009 s, DX12 4.685 s, **3.204×** speedup |

Both explicit adapter filters also pass the opt-in media DX12 conformance gate, including video/image rotation/clip/shapes and title/Japanese callout CPU parity within one RGBA value. The corrected continuous-play NVIDIA one-minute release rehearsal reached 59.7 s and passed with 25 telemetry samples: 0.1 MiB WS growth, 0.1 MiB private growth, and 0 MiB dedicated-VRAM growth after a 10 s warm-up.

## Hardware matrix

| Vendor | Adapter/filter | Current availability | Debug smoke | Release 1080p benchmark | 30m soak | Evidence |
|---|---|---|---|---|---|---|
| NVIDIA | `NVIDIA GeForce RTX 5070 Ti` / `NVIDIA` | Detected | PASS | PASS (4.531× vs CPU) | 1m rehearsal PASS; 30m未実行 | `%TEMP%\lattice-studio-smoke`, benchmark result JSON |
| AMD | `AMD Radeon(TM) Graphics` / `AMD` | Detected | PASS | PASS (3.204× vs CPU) | 未実行 | `%TEMP%\lattice-studio-smoke`, benchmark result JSON |
| Intel | `Intel` | Unavailable in the current environment | 外部 hardware pending | 外部 hardware pending | 外部 hardware pending | No adapter/evidence yet |

For every executed row, record:

- exact commands and debug/release profile;
- CLI JSON `renderer.adapter`, output spec, elapsed seconds, and speedup;
- ffprobe width, height, and average frame rate;
- Studio exit status plus log assertions;
- post-warm-up maximum WS/private/VRAM growth;
- any typed renderer failure `phase`, `kind`, `stage`, and reason.
