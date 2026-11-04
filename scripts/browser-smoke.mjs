// Optional test tool. Install playwright, or set SURFACE_PLAYWRIGHT to its ESM entry.
import { mkdir, writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import { startWebServer } from './web-test-support.mjs';
const { chromium } = await import(process.env.SURFACE_PLAYWRIGHT || 'playwright');
const browser = await chromium.launch({
  headless: true,
  ...(process.env.SURFACE_CHROME ? { executablePath: process.env.SURFACE_CHROME } : {}),
  args: ['--enable-unsafe-webgpu'],
});
const web = await startWebServer();
try {
  const page = await browser.newPage({ viewport: { width: 1000, height: 900 }, deviceScaleFactor: 2 });
  const errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await page.goto(web.url);
  await page.waitForFunction(() => document.querySelector('#status').textContent.startsWith('Ready'), { timeout: 45000 });
  assert.equal(await page.evaluate(() => !!navigator.gpu), true);
  await page.waitForFunction(() => {
    const canvas = document.querySelector('canvas');
    const root = window.showcase.control_bounds('surface-page');
    return root.length === 4 && Math.abs(root[2] - canvas.clientWidth) <= 1
      && canvas.width === Math.round(canvas.clientWidth * devicePixelRatio);
  });
  const inspect = () => page.evaluate(() => window.showcase.inspect());
  const settle = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const click = async id => {
    await page.waitForFunction(id => window.showcase.control_bounds(id).length === 4, id);
    const rect = await page.evaluate(id => Array.from(window.showcase.control_bounds(id)), id);
    assert.equal(rect.length, 4, `missing semantic control: ${id}`);
    const box = await page.locator('canvas').boundingBox();
    await page.mouse.click(box.x + rect[0] + rect[2] / 2, box.y + rect[1] + rect[3] / 2);
  };
  await click('increment');
  await page.waitForFunction(() => window.showcase.inspect().startsWith('count=1;'));
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => window.showcase.inspect().startsWith('count=2;'));
  await page.keyboard.press('Tab');
  await page.waitForFunction(() => window.showcase.inspect().endsWith('focused=reset'));
  await page.keyboard.press('Tab');
  await page.waitForFunction(() => window.showcase.inspect().endsWith('focused=name'));
  await page.keyboard.type('Chris');
  await page.waitForFunction(() => window.showcase.inspect().includes('name=Chris;'));
  await page.keyboard.press('Backspace');
  await page.waitForFunction(() => window.showcase.inspect().includes('name=Chri;'));
  await click('dark-mode');
  await page.waitForFunction(() => window.showcase.inspect().includes('dark=true;'));
  await click('disabled');
  await settle();
  assert.match(await inspect(), /^count=2;/);
  await mkdir('artifacts', { recursive: true });
  await page.screenshot({ path: 'artifacts/browser-showcase-wide.png' });
  await page.setViewportSize({ width: 480, height: 600 });
  await page.waitForFunction(() => window.showcase.control_bounds('surface-page')[2] === 480);
  await click('increment');
  await page.waitForFunction(() => window.showcase.inspect().startsWith('count=3;'));
  await page.waitForFunction(() => window.showcase.control_bounds('title').length === 4);
  const beforeScroll = await page.evaluate(() => window.showcase.control_bounds('title')[1]);
  await page.mouse.move(200, 450);
  await page.mouse.wheel(0, 350);
  await page.waitForFunction(y => window.showcase.control_bounds('title')[1] < y, beforeScroll);
  await settle();
  assert.deepEqual(errors, [], errors.join('\n'));
  assert.match(await page.locator('#status').textContent(), /^Ready/);
  await page.screenshot({ path: 'artifacts/browser-showcase.png' });
  const adapter = await page.evaluate(async () => {
    const adapter = await navigator.gpu.requestAdapter();
    return adapter ? { vendor: adapter.info.vendor, architecture: adapter.info.architecture, device: adapter.info.device, description: adapter.info.description, isFallbackAdapter: adapter.info.isFallbackAdapter } : null;
  });
  const evidence = { browser: await browser.version(), state: await inspect(), errors, adapter, viewport: page.viewportSize() };
  await writeFile('artifacts/browser-smoke.json', JSON.stringify(evidence, null, 2) + '\n');
  console.log(JSON.stringify(evidence));
} finally {
  await browser.close();
  await web.close();
}
