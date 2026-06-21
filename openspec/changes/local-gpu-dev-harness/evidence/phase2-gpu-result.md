# Phase 2 GPU result (box 4.3)

Recorded: 2026-06-21T03:46:46.672Z
Source probe: /home/llambit/0_repos/alephFe/scratch/evidence/20260620-224640-container-5.4/gpu-probe.json
Served URL: http://localhost:8090  (headless=true)

## WebGL2 (reliable target)
- Renderer: ANGLE (NVIDIA, Vulkan 1.4.312 (NVIDIA Tesla P40 (0x00001B38)), NVIDIA)
- Vendor: Google Inc. (NVIDIA)
- Classification: **hardware-nvidia**

## WebGPU (stretch)
- Adapter present: **NO (requestAdapter -> NULL)**
- Adapter info: (none)
- Classification: none

## Verdict
- Probe verdict: **HARDWARE**
- Outcome: WebGPU did NOT initialize -> settle on hardware WebGL2 (known Pascal/Maxwell limit, not a harness failure).

## chrome://gpu cross-check (paste manually)
Open chrome://gpu in the headed session and paste the Graphics Feature Status
block (WebGL / WebGL2 / WebGPU / Vulkan lines) below:

```
(paste chrome://gpu Graphics Feature Status here)
```
