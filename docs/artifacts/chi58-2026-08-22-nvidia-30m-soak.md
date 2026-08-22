# CHI-58 NVIDIA 30-minute release soak — 2026-08-22

Durable copy of the local Windows DX12 soak that previously lived only under `%TEMP%\lattice-studio-smoke`.

## Environment

| Field | Value |
|---|---|
| Date | 2026-08-22 (JST) |
| Working tree | `679f91460460ec81f52744fa0bc7f75be5fa4d31` (`feat/alpha-studio`; local was 6 Linux-smoke commits behind `origin/feat/alpha-studio`) |
| OS | Windows 11 Home 25H2, build `26200.9168` |
| Adapter | NVIDIA GeForce RTX 5070 Ti |
| Driver | `32.0.16.1047` |
| Profile | release |

## Command

```powershell
./scripts/studio-smoke.ps1 -Release -Renderer gpu-dx12 -Adapter "NVIDIA" -SoakMinutes 30 `
  -SoakWarmupSeconds 30 `
  -MaxWorkingSetGrowthMB 512 `
  -MaxPrivateMemoryGrowthMB 512 `
  -MaxDedicatedGpuGrowthMB 512
```

## Result

`SMOKE OK` pid `55152` exit `0`.

| Gate | Observed | Limit |
|---|---|---|
| Renderer | `requested=require_gpu_dx12`, `active=gpu_dx12`, adapter=`NVIDIA GeForce RTX 5070 Ti` | no CPU fallback |
| Audio monitor | `audio monitor explicitly disabled` | required in soak mode |
| Preview | in-memory `640x360` frames; last timestamp **1799.6 s** | ≥ 1795 s |
| Distinct preview times | 17993 frame lines | ≥ 3 |
| Panic / recreate / PNG cache | none | fail on any |
| Smoke watchdog | `smoke quit` | required |

Post-warm-up growth (874 samples after 30 s; raw 1 Hz rows in the CSV):

| Signal | Growth | Limit |
|---|---:|---:|
| Working set | **0.5 MiB** | 512 MiB |
| Private memory | **0.1 MiB** | 512 MiB |
| Dedicated GPU | **0 MiB** | 512 MiB |

First measured sample after warm-up (`elapsed_seconds=30.3`): WS `218353664`, private `606208000`, dedicated GPU `433778688`. Last sample (`1799.9`): WS `218730496`, private `606072832`, dedicated GPU `433778688`.

## Artifacts

- [chi58-2026-08-22-nvidia-30m-soak.telemetry.csv](chi58-2026-08-22-nvidia-30m-soak.telemetry.csv) — full per-second WS / private / dedicated GPU rows
- [chi58-2026-08-22-nvidia-30m-soak.log](chi58-2026-08-22-nvidia-30m-soak.log) — Studio log with the 17993-line preview-frame stream omitted; first and last eight frames kept

Original TEMP paths: `studio-smoke-20260822-090326.log` and `studio-smoke-20260822-090326.telemetry.csv`.
