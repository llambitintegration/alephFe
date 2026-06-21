# IO GPU thermal / cooling validation (2026-06-20)

Context: the GPU-passthrough container path (now the primary path) runs sustained
GPU workloads on IO's **passively cooled** Tesla P40s (`fan = N/A`; they rely on
chassis airflow). Before depending on them we validated cooling under load. Hard
abort armed at **65 °C** throughout.

Hardware: 2× Tesla P40 (GPU 0, GPU 1), 1× Quadro M2000 (GPU 2). Driver 580.
Idle: P40s 29–30 °C, Quadro 40 °C (active fan ~56%).

## Test 1 — single P40, FP32 ALU stress (90 s)
- GPU 0, 100% util, ~106 W (pure FMA under-draws power).
- Baseline 29 °C → **peak 43 °C**, plateaued ~43 °C. No abort.

## Test 2 — dual P40, cuBLAS SGEMM full burn (300 s)
Worst case for shared chassis airflow: both passive cards at once.
- Both GPUs 100% util, **~165 W each** (realistic FP32 GEMM load; SGEMM does not
  hit the 250 W TDP cap — that needs a synthetic power-virus / INT8, which no real
  workload here produces).
- Reached thermal **steady state** ~2 min in and held:
  - GPU 0: baseline 30 °C → **peak 54 °C** (flat ~54 from ~180 s)
  - GPU 1: baseline 30 °C → **peak 57 °C** (flat ~57 from ~190 s)
- Completed the full 5-minute soak with **no abort**.

## Verdict
**Cooling is adequate** for how these cards will be used. Margins at steady state:
**8–11 °C below the 65 °C abort** and **~28–31 °C below the ~85 °C throttle**.
The ~3 °C GPU 0→1 gap is consistent with airflow ordering in the chassis.

Not probed: the absolute ~250 W power-virus ceiling (not representative of the
marathon-web GPU harness or any normal compute workload).

Tooling: `scratch/gpu-temp-test.sh` (single, FP32 ALU) and
`scratch/gpu-burn-full.sh` (multi-GPU, cuBLAS GEMM), both with an in-script hard
abort. Raw per-2 s CSV logs were written under `scratch/evidence/` (gitignored);
key figures captured above.
