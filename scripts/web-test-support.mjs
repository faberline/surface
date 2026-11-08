// Owned by each test process, so an interrupted run leaves no fixture daemon.
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { dirname, extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

export async function startWebServer(example = 'showcase') {
  if (process.env.SURFACE_URL) return { url: process.env.SURFACE_URL, close: async () => {} };
  // Output of `cargo xtask web build <app>`. SURFACE_WEB_PROFILE=dev selects
  // a `--dev` build; SURFACE_WEB_DIR selects any built directory.
  const profile = process.env.SURFACE_WEB_PROFILE || 'release';
  const root = resolve(process.env.SURFACE_WEB_DIR || resolve(dirname(fileURLToPath(import.meta.url)), `../target/web/${profile}/${example}`));
  const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json' };
  const server = createServer(async (req, res) => {
    try {
      if (!['GET', 'HEAD'].includes(req.method)) { res.writeHead(405).end(); return; }
      const path = decodeURIComponent(new URL(req.url, 'http://localhost').pathname);
      const file = resolve(root, '.' + (path === '/' ? '/index.html' : path));
      if (!file.startsWith(root + sep)) { res.writeHead(403).end(); return; }
      const data = await readFile(file);
      res.writeHead(200, { 'content-type': types[extname(file)] || 'application/octet-stream', 'content-length': data.length });
      res.end(req.method === 'HEAD' ? undefined : data);
    } catch { res.writeHead(404).end(); }
  });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  return { url: `http://127.0.0.1:${server.address().port}`, close: () => new Promise((resolve, reject) => server.close(e => e ? reject(e) : resolve())) };
}
