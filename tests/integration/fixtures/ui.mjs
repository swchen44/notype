import { mkdir } from "node:fs/promises";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
const require = createRequire(
  new URL("../../../desktop/package.json", import.meta.url),
);
const { chromium } = require("playwright");
const { createServer } = await import(
  new URL(
    "../../../desktop/node_modules/vite/dist/node/index.js",
    import.meta.url,
  )
);
export const desktop = fileURLToPath(
  new URL("../../../desktop", import.meta.url),
);

/** Fresh browser + real Vite/React/i18next; the caller stubs only public IPC. */
export async function uiFixture(t) {
  await mkdir(new URL("../../../.local", import.meta.url), { recursive: true });
  const server = await createServer({
    root: desktop,
    server: { host: "127.0.0.1", port: 0, strictPort: false },
  });
  await server.listen();
  t.after(() => server.close());
  const browser = await chromium.launch({
    executablePath:
      process.env.NOTYPE_TEST_BROWSER ??
      (process.platform === "win32"
        ? "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"
        : undefined),
    headless: true,
  });
  t.after(() => browser.close());
  const page = await browser.newPage({ viewport: { width: 900, height: 900 } });
  const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
  const html = await server.transformIndexHtml(
    "/",
    '<html><head></head><body><div id="root"></div></body></html>',
  );
  await page.route(origin + "/", (route) =>
    route.fulfill({ contentType: "text/html", body: html }),
  );
  await page.goto(origin);
  return page;
}
