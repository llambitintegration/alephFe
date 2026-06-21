import { test, expect } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

// GPU verification probe (boxes 2.1/2.2). Against the served marathon-web app it
// reports the WebGL2 unmasked VENDOR/RENDERER and the WebGPU
// requestAdapter()-vs-NULL result, classifies software (SwiftShader/llvmpipe) vs
// hardware (NVIDIA/other), and writes JSON + a screenshot to EVID_DIR. With
// EXPECT_HARDWARE set it FAILS on a silent software fallback — this is what makes
// a "real GPU" claim non-fakeable and gates the container path (box 5.3).

type WebglInfo = { version: string; vendor: string; renderer: string } | null;
type WebgpuInfo = {
  supported: boolean;
  adapter: boolean;
  info: { vendor?: string; architecture?: string; device?: string; description?: string } | null;
};

function classify(
  renderer: string | undefined,
): 'software' | 'hardware-nvidia' | 'hardware-other' | 'unknown' {
  const r = (renderer || '').toLowerCase();
  if (!r) return 'unknown';
  if (/swiftshader|llvmpipe|software|microsoft basic|softpipe/.test(r)) return 'software';
  if (/nvidia|geforce|quadro|tesla|rtx|gtx/.test(r)) return 'hardware-nvidia';
  if (/intel|amd|radeon|apple|mesa|adreno|mali/.test(r)) return 'hardware-other';
  return 'unknown';
}

test('gpu-probe', async ({ page }, testInfo) => {
  const consoleErrors: string[] = [];
  page.on('console', (msg) => {
    if (msg.type() === 'error') consoleErrors.push(msg.text());
  });

  await page.goto('/');
  await page
    .locator('#loading')
    .waitFor({ state: 'hidden', timeout: 60_000 })
    .catch(() => {
      // A broken adapter may never hide #loading; the probe queries the graphics
      // stack directly below, so don't hard-fail on the loading overlay.
    });
  await page.waitForTimeout(1500);

  const probe = await page.evaluate(async () => {
    const out: { webgl: WebglInfo; webgpu: WebgpuInfo } = {
      webgl: null,
      webgpu: { supported: false, adapter: false, info: null },
    };
    try {
      const c = document.createElement('canvas');
      const gl = (c.getContext('webgl2') || c.getContext('webgl')) as WebGLRenderingContext | null;
      if (gl) {
        const dbg = gl.getExtension('WEBGL_debug_renderer_info');
        out.webgl = {
          version: String(gl.getParameter(gl.VERSION)),
          vendor: String(
            dbg ? gl.getParameter(dbg.UNMASKED_VENDOR_WEBGL) : gl.getParameter(gl.VENDOR),
          ),
          renderer: String(
            dbg ? gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER),
          ),
        };
      }
    } catch {
      /* leave webgl null */
    }
    try {
      const gpu = (navigator as unknown as { gpu?: any }).gpu;
      if (gpu) {
        out.webgpu.supported = true;
        const adapter = await gpu.requestAdapter();
        if (adapter) {
          out.webgpu.adapter = true;
          let info: any = adapter.info || null;
          if (!info && typeof adapter.requestAdapterInfo === 'function') {
            try {
              info = await adapter.requestAdapterInfo();
            } catch {
              /* ignore */
            }
          }
          out.webgpu.info = info
            ? {
                vendor: info.vendor,
                architecture: info.architecture,
                device: info.device,
                description: info.description,
              }
            : null;
        }
      }
    } catch {
      /* leave webgpu defaults */
    }
    return out;
  });

  const webglClass = classify(probe.webgl?.renderer);
  const webgpuDesc = probe.webgpu.info?.description || probe.webgpu.info?.architecture || '';
  const webgpuClass = probe.webgpu.adapter ? classify(webgpuDesc) || 'unknown' : 'none';

  const adapterErrors = consoleErrors.filter(
    (e) => e.includes('No available adapters') || e.includes('No GPU adapter'),
  );

  const summary = {
    timestamp: new Date().toISOString(),
    baseURL: testInfo.project.use.baseURL,
    headless: testInfo.project.use.headless,
    webgl: { ...probe.webgl, classification: webglClass },
    webgpu: {
      supported: probe.webgpu.supported,
      adapterPresent: probe.webgpu.adapter,
      info: probe.webgpu.info,
      classification: webgpuClass,
    },
    consoleAdapterErrors: adapterErrors,
    verdict: webglClass.startsWith('hardware')
      ? 'HARDWARE'
      : webglClass === 'software'
        ? 'SOFTWARE (SwiftShader/llvmpipe fallback)'
        : 'UNKNOWN',
  };

  const evidDir = process.env.EVID_DIR || path.join(__dirname, 'evidence');
  fs.mkdirSync(evidDir, { recursive: true });
  fs.writeFileSync(path.join(evidDir, 'gpu-probe.json'), JSON.stringify(summary, null, 2));
  await page.screenshot({ path: path.join(evidDir, 'gpu-probe.png'), fullPage: false }).catch(() => {});

  console.log('\n===================== GPU PROBE =====================');
  console.log(`URL            : ${summary.baseURL}  (headless=${summary.headless})`);
  console.log(`WebGL2 renderer: ${probe.webgl?.renderer ?? '(none)'}`);
  console.log(`WebGL2 vendor  : ${probe.webgl?.vendor ?? '(none)'}`);
  console.log(`WebGL class    : ${webglClass}`);
  console.log(`WebGPU adapter : ${probe.webgpu.adapter ? 'PRESENT' : 'NULL'}`);
  if (probe.webgpu.info) console.log(`WebGPU info    : ${JSON.stringify(probe.webgpu.info)}`);
  console.log(`VERDICT        : ${summary.verdict}`);
  console.log(`Evidence       : ${evidDir}`);
  console.log('=====================================================\n');

  if (['1', 'true', 'yes', 'on'].includes((process.env.EXPECT_HARDWARE || '').toLowerCase())) {
    expect(webglClass, `Expected a hardware GPU but got: ${probe.webgl?.renderer}`).toMatch(
      /^hardware/,
    );
  }
});
