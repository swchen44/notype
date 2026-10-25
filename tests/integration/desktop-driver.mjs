import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve, basename, sep } from "node:path";
import { createServer } from "node:net";
import { once } from "node:events";
import { setTimeout as delay } from "node:timers/promises";
import { chromium } from "../../desktop/node_modules/playwright/index.mjs";

export async function desktopFixture(options = {}) {
  assert.equal(process.platform, "win32");
  assert.ok(
    process.env.NOTYPE_DESKTOP_EXE,
    "Set NOTYPE_DESKTOP_EXE to the actual debug executable",
  );
  const executable = resolve(process.env.NOTYPE_DESKTOP_EXE);
  const directory = await mkdtemp(join(tmpdir(), "notype-workflow-receipt-"));
  const bindings = Object.fromEntries(
    [
      "transcribe",
      "translation",
      "ask",
      "stop",
      "transcribe_with_post_process",
      "cancel",
    ].map((id) => [
      id,
      {
        id,
        name: id,
        description: "nonsecret isolated receipt",
        default_binding: "",
        current_binding: options.bindings?.[id] ?? "",
      },
    ]),
  );
  await writeFile(
    join(directory, "settings_store.json"),
    JSON.stringify({
      settings: {
        bindings,
        always_on_microphone: false,
        selected_model: "",
        onboarding_completed: true,
        show_whats_new_on_update: false,
        app_language: "zh-TW",
      },
    }),
  );
  const running = new Set();
  async function close(app) {
    if (!app || !running.delete(app)) return;
    await app.browser.close();
    if (app.child.exitCode === null) {
      app.child.kill();
      await once(app.child, "exit");
    }
  }
  async function launch() {
    const reservation = createServer();
    reservation.listen(0, "127.0.0.1");
    await once(reservation, "listening");
    const port = reservation.address().port;
    await new Promise((done) => reservation.close(done));
    let logs = "";
    const child = spawn(executable, ["--start-hidden"], {
      env: {
        ...process.env,
        NOTYPE_DATA_DIR: directory,
        NOTYPE_DEBUG_WEBVIEW_PORT: String(port),
      },
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });
    child.stdout.on("data", (chunk) => {
      logs += chunk;
    });
    child.stderr.on("data", (chunk) => {
      logs += chunk;
    });
    let browser;
    try {
      for (let attempt = 0; attempt < 90; attempt++) {
        assert.equal(
          child.exitCode,
          null,
          `Desktop exited: ${logs.slice(-4000)}`,
        );
        try {
          browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
          break;
        } catch {
          await delay(300);
        }
      }
      assert.ok(browser, `No native CDP endpoint: ${logs.slice(-4000)}`);
      let page;
      for (let attempt = 0; attempt < 60; attempt++) {
        page = browser
          .contexts()
          .flatMap((context) => context.pages())
          .find((candidate) => candidate.url().includes("tauri.localhost"));
        if (page) break;
        await delay(100);
      }
      assert.ok(
        page,
        `native main WebView; owned pages=${JSON.stringify(
          browser
            .contexts()
            .flatMap((context) => context.pages())
            .map((page) => page.url()),
        )}; startup=${logs.slice(-2500)}`,
      );
      page.setDefaultTimeout(5000);
      await page.waitForFunction(() => window.__TAURI_INTERNALS__?.invoke);
      if (process.env.NOTYPE_NATIVE_MODULE_RECEIPT) {
        const moduleQuery = `$notypeFixtureModules = @((Get-Process -Id ${child.pid}).Modules | Where-Object { $_.ModuleName -match '^(DirectML|onnxruntime).*\\.dll$' } | ForEach-Object { @{ name=$_.ModuleName; path=$_.FileName; version=$_.FileVersionInfo.FileVersion } }); ConvertTo-Json -InputObject $notypeFixtureModules -Compress`;
        let observation;
        try {
          observation = {
            status: "observed-after-native-startup",
            modules: JSON.parse(
              execFileSync(
                "powershell.exe",
                ["-NoProfile", "-Command", moduleQuery],
                { windowsHide: true, encoding: "utf8" },
              ),
            ),
          };
        } catch {
          observation = { status: "module-enumeration-unavailable" };
        }
        await writeFile(
          process.env.NOTYPE_NATIVE_MODULE_RECEIPT,
          JSON.stringify(observation, null, 2),
        );
      }
      const app = { child, browser, page, logs: () => logs };
      running.add(app);
      return app;
    } catch (error) {
      await browser?.close();
      child.kill();
      throw error;
    }
  }
  return {
    directory,
    launch,
    close,
    async dispose() {
      for (const app of running) await close(app);
      const owned = resolve(directory);
      assert.ok(
        owned.startsWith(resolve(tmpdir()) + sep) &&
          basename(owned).startsWith("notype-workflow-receipt-"),
        "Only remove this fixture’s resolved temporary directory",
      );
      await rm(owned, {
        recursive: true,
        force: true,
        maxRetries: 10,
        retryDelay: 250,
      });
    },
  };
}
export const invoke = (page, command, args = {}) =>
  page.evaluate(
    ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
    { command, args },
  );
