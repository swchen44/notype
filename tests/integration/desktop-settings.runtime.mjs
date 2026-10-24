import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { createServer } from 'node:net';
import { once } from 'node:events';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium } from '../../desktop/node_modules/playwright/index.mjs';

// Explicit opt-in: starts the actual desktop executable, without recording.
test('desktop settings survive restart externally and reject submission/updater enablement', { timeout: 180000 }, async () => {
  assert.equal(process.platform, 'win32', 'This receipt currently targets Windows amd64');
  assert.ok(process.env.NOTYPE_DESKTOP_EXE, 'Set NOTYPE_DESKTOP_EXE to a no-bundle native build');
  const executable = resolve(process.env.NOTYPE_DESKTOP_EXE);
  const directory = await mkdtemp(join(tmpdir(), 'notype-desktop-receipt-'));
  const bindings = Object.fromEntries(['transcribe', 'transcribe_with_post_process', 'cancel'].map(id => [id, {
    id, name: id, description: 'Runtime receipt: no keyboard shortcut', default_binding: '', current_binding: '',
  }]));
  // Seed safe launch conditions; assertions use the app's public IPC boundary.
  await writeFile(join(directory, 'settings_store.json'), JSON.stringify({ settings: {
    bindings, always_on_microphone: false, selected_model: '', onboarding_completed: false,
    auto_submit: true, update_checks_enabled: true, app_language: 'en',
  }}));
  async function launch() {
    const server = createServer();
    server.listen(0, '127.0.0.1');
    await once(server, 'listening');
    const port = server.address().port;
    await new Promise(done => server.close(done));
    let logs = '';
    const child = spawn(executable, ['--start-hidden'], {
      env: { ...process.env, NOTYPE_DATA_DIR: directory,
        NOTYPE_DEBUG_WEBVIEW_PORT: String(port),
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}` },
      windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'],
    });
    child.stdout.on('data', chunk => { logs += chunk; });
    child.stderr.on('data', chunk => { logs += chunk; });
    let browser;
    let connectionError;
    try {
      for (let attempt = 0; attempt < 90; attempt++) {
        assert.equal(child.exitCode, null, `Desktop exited: ${logs.slice(-4000)}`);
        try { browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`); break; }
        catch (error) { connectionError = error.message; await delay(500); }
      }
      assert.ok(browser, `No WebView2 CDP endpoint (${connectionError}): ${logs.slice(-4000)}`);
      let page;
      for (let attempt = 0; attempt < 60; attempt++) {
        page = browser.contexts().flatMap(context => context.pages()).find(candidate => candidate.url().includes('tauri.localhost'));
        if (page) break;
        await delay(250);
      }
      assert.ok(page, 'The notype main WebView exists');
      await page.waitForFunction(() => window.__TAURI_INTERNALS__?.invoke);
      return { child, browser, page };
    } catch (error) {
      await browser?.close();
      child.kill();
      throw error;
    }
  }
  const invoke = (page, command, args = {}) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args });
  async function close(app) {
    await app.browser.close();
    if (app.child.exitCode === null) {
      app.child.kill();
      await once(app.child, 'exit');
    }
  }
  let app;
  try {
    app = await launch();
    assert.equal(await app.page.title(), 'notype');
    await app.page.getByRole('img', { name: 'notype', exact: true }).waitFor();
    assert.equal(await invoke(app.page, 'get_app_dir_path'), directory);
    const initial = await invoke(app.page, 'get_app_settings');
    assert.equal(initial.always_on_microphone, false);
    assert.equal(initial.auto_submit, false);
    assert.equal(initial.update_checks_enabled, false);
    assert.ok(Object.values(initial.bindings).every(binding => binding.current_binding === ''));
    assert.equal(await invoke(app.page, 'is_update_checks_locked'), true);
    await assert.rejects(invoke(app.page, 'change_auto_submit_setting', { enabled: true }), /manual submission/);
    await assert.rejects(invoke(app.page, 'change_update_checks_setting', { enabled: true }), /npm/);
    await assert.rejects(invoke(app.page, 'plugin:updater|check'), /not found|not allowed|unknown|denied/i);
    await invoke(app.page, 'change_app_language_setting', { language: 'zh-TW' });
    assert.equal((await invoke(app.page, 'get_app_settings')).app_language, 'zh-TW');
    // Public store autosave defaults to 100 ms.
    await delay(1000);
    await close(app);
    app = await launch();
    const restored = await invoke(app.page, 'get_app_settings');
    assert.equal(restored.app_language, 'zh-TW');
    assert.equal(restored.auto_submit, false);
    assert.equal(restored.update_checks_enabled, false);
    await close(app);
    app = undefined;
  } finally {
    if (app) await close(app);
    await rm(directory, { recursive: true, force: true, maxRetries: 10, retryDelay: 250 });
  }
});
