import assert from "node:assert/strict";
import test from "node:test";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { createServer as reservePort } from "node:net";
import { once } from "node:events";
const require = createRequire(
  new URL("../../desktop/package.json", import.meta.url),
);
const { chromium } = require("playwright");
const { createServer } = await import(
  new URL("../../desktop/node_modules/vite/dist/node/index.js", import.meta.url)
);

async function fixture(
  t,
  { pendingRegistration = false, strict = false } = {},
) {
  const reservation = reservePort();
  reservation.listen(0, "127.0.0.1");
  await once(reservation, "listening");
  const port = reservation.address().port;
  await new Promise((resolve) => reservation.close(resolve));
  const server = await createServer({
    cacheDir: fileURLToPath(
      new URL("../../.local/recording-elapsed-vite", import.meta.url),
    ),
    plugins: [
      {
        name: "overlay-rendered-fixture",
        resolveId(id) {
          if (id === "/__overlay_fixture.tsx") return id;
        },
        load(id) {
          if (id === "/__overlay_fixture.tsx")
            return 'export {default as React} from "react"; export {createRoot} from "react-dom/client"; export {default as Overlay} from "/src/overlay/RecordingOverlay.tsx";';
        },
      },
    ],
    root: fileURLToPath(new URL("../../desktop", import.meta.url)),
    server: { host: "127.0.0.1", port, strictPort: true },
  });
  t.after(() => server.close());
  await server.listen();
  const browser = await chromium.launch({
    executablePath:
      process.env.NOTYPE_TEST_BROWSER ??
      (process.platform === "win32"
        ? "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"
        : undefined),
    headless: true,
  });
  t.after(() => browser.close());
  const page = await browser.newPage();
  page.setDefaultTimeout(3000);
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
  const html = await server.transformIndexHtml(
    "/",
    '<html><body><div id="root"></div></body></html>',
  );
  await page.route(origin + "/", (route) =>
    route.fulfill({ contentType: "text/html", body: html }),
  );
  await page.goto(origin);
  await page.evaluate(
    async ({ pendingRegistration, strict }) => {
      let next = 0;
      const callbacks = new Map();
      const listeners = new Map();
      const intervals = new Map();
      const pending = [];
      const settings = [];
      const disposed = [];
      window.overlayFixture = {
        now: 10000,
        deferSettings: false,
        emit(event, payload) {
          for (const [id, item] of listeners)
            if (item.event === event)
              callbacks.get(item.handler)?.({ event, id, payload });
        },
        advance(ms) {
          this.now += ms;
          for (const callback of intervals.values()) callback();
        },
        counts() {
          return {
            listeners: [...listeners.values()].map((item) => item.event),
            intervals: intervals.size,
            pending: pending.length,
            disposed: [...disposed],
          };
        },
        settleRegistrations(reject = false, limit = pending.length) {
          for (const request of pending.splice(0, limit)) {
            if (reject) {
              listeners.delete(request.id);
              request.reject(new Error("registration fixture rejection"));
            } else request.resolve(request.id);
          }
        },
        settleSettings(
          position = "top",
          reject = false,
          limit = settings.length,
        ) {
          for (const request of settings.splice(0, limit))
            reject
              ? request.reject(new Error("settings fixture rejection"))
              : request.resolve({
                  app_language: "en",
                  overlay_position: position,
                });
        },
      };
      Object.defineProperty(performance, "now", {
        value: () => window.overlayFixture.now,
      });
      window.setInterval = (callback) => {
        const id = ++next;
        intervals.set(id, callback);
        return id;
      };
      window.clearInterval = (id) => intervals.delete(id);
      window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
      window.__TAURI_INTERNALS__ = {
        transformCallback(callback) {
          const id = ++next;
          callbacks.set(id, callback);
          return id;
        },
        invoke(command, args) {
          if (command === "plugin:event|listen") {
            const id = ++next;
            listeners.set(id, args);
            if (pendingRegistration)
              return new Promise((resolve, reject) =>
                pending.push({ id, resolve, reject }),
              );
            return Promise.resolve(id);
          }
          if (command === "plugin:event|unlisten") {
            listeners.delete(args.eventId);
            disposed.push(args.eventId);
            return Promise.resolve();
          }
          if (command === "get_app_settings")
            return window.overlayFixture.deferSettings
              ? new Promise((resolve, reject) =>
                  settings.push({ resolve, reject }),
                )
              : Promise.resolve({
                  app_language: "en",
                  overlay_position: "bottom",
                });
          return Promise.resolve();
        },
      };
      const { React, createRoot, Overlay } = await import(
        "/__overlay_fixture.tsx"
      );
      let root;
      window.overlayFixture.mount = () => {
        root = createRoot(document.getElementById("root"));
        root.render(
          React.createElement(
            strict ? React.StrictMode : React.Fragment,
            null,
            React.createElement(Overlay),
          ),
        );
      };
      window.overlayFixture.unmount = () => root.unmount();
      window.overlayFixture.mount();
    },
    { pendingRegistration, strict },
  );
  await page.waitForFunction(() =>
    window.overlayFixture.counts().listeners.includes("show-overlay"),
  );
  const emit = (event, payload) =>
    page.evaluate(
      ({ event, payload }) => window.overlayFixture.emit(event, payload),
      { event, payload },
    );
  return {
    page,
    emit,
    errors,
    advance: (ms) =>
      page.evaluate((ms) => window.overlayFixture.advance(ms), ms),
  };
}

const show = (captureId = "1", revision = "1", state = "recording") => ({
  captureId,
  revision,
  state,
});
const ready = (captureId = "1", captureRevision = "1") => ({
  captureId,
  captureRevision,
});

test("compact ready capture displays digits from local elapsed rather than interval ticks", async (t) => {
  const { page, emit, advance, errors } = await fixture(t);
  await page.waitForFunction(
    () => window.overlayFixture.counts().listeners.length === 6,
  );
  await emit("show-overlay", show());
  await emit("recording-ready", ready());
  await page.locator(".stimer").waitFor();
  assert.equal(await page.locator(".stimer").textContent(), "00:00");
  await advance(12000);
  await page.waitForFunction(
    () => document.querySelector(".stimer")?.textContent === "00:12",
  );
  assert.equal(
    await page
      .locator(".stimer")
      .evaluate((element) => getComputedStyle(element).opacity),
    "1",
  );
  assert.equal(await page.locator(".stimer").getAttribute("aria-live"), "off");
  const layout = await page.evaluate(() => {
    const wave = document.querySelector(".swave").getBoundingClientRect();
    const timer = document.querySelector(".stimer").getBoundingClientRect();
    const cancel = document.querySelector(".sx").getBoundingClientRect();
    return {
      waveformRight: wave.right,
      timerLeft: timer.left,
      timerRight: timer.right,
      cancelLeft: cancel.left,
    };
  });
  assert.ok(
    layout.waveformRight <= layout.timerLeft,
    `waveform and digits must not overlap: ${JSON.stringify(layout)}`,
  );
  assert.ok(layout.timerRight <= layout.cancelLeft);

  assert.deepEqual(errors, []);
});

test("streaming timer is visible with or without text and jumps across a minute without duplicate resets", async (t) => {
  const { page, emit, advance } = await fixture(t);
  await page.waitForFunction(
    () => window.overlayFixture.counts().listeners.length === 6,
  );
  await emit("show-overlay", show("1", "1", "streaming"));
  assert.equal(
    await page.locator(".stimer").count(),
    0,
    "arming has no running timer",
  );
  await advance(5000);
  await emit("recording-ready", ready());
  await page.locator(".stimer").waitFor();
  assert.equal(
    await page.locator(".stimer").textContent(),
    "00:00",
    "delayed receipt starts display elapsed locally",
  );
  await advance(59000);
  await page.waitForFunction(
    () => document.querySelector(".stimer")?.textContent === "00:59",
  );
  await emit("recording-ready", ready());
  await emit("show-overlay", show("1", "1", "streaming"));
  await advance(1000);
  await page.waitForFunction(
    () => document.querySelector(".stimer")?.textContent === "01:00",
  );
  await emit("stream-text-event", {
    capture_id: "1",
    committed: "fixture words",
    tentative: "",
  });
  await page.locator(".scard.open").waitFor();
  assert.equal(await page.locator(".stimer").textContent(), "01:00");
  assert.equal(
    await page.evaluate(() => window.overlayFixture.counts().intervals),
    1,
  );
});

test("old ready hide phase and text cannot change a successor, working and terminal stop its clock", async (t) => {
  const { page, emit, advance } = await fixture(t);
  await page.waitForFunction(
    () => window.overlayFixture.counts().listeners.length === 6,
  );
  await emit("show-overlay", show());
  await emit("recording-ready", ready());
  await emit("show-overlay", show("2", "2", "streaming"));
  await emit("recording-ready", ready("2", "2"));
  await advance(12000);
  await emit("recording-ready", ready());
  await emit("hide-overlay", {
    captureId: "1",
    revision: "4",
    reason: "finished",
  });
  await emit("stream-phase-event", {
    capture_id: "1",
    revision: "5",
    phase: "working",
    kind: "transcribing",
  });
  await emit("stream-text-event", {
    capture_id: "1",
    committed: "stale words",
    tentative: "",
  });
  await page.waitForFunction(
    () => document.querySelector(".stimer")?.textContent === "00:12",
  );
  assert.equal(await page.getByText("stale words").count(), 0);
  await emit("stream-phase-event", {
    capture_id: "2",
    revision: "6",
    phase: "working",
    kind: "transcribing",
  });
  await page.locator(".sspinner").waitFor();
  assert.equal(await page.locator(".stimer").count(), 0);
  await advance(10000);
  await emit("recording-ready", ready("2", "2"));
  await emit("show-overlay", show("2", "7", "streaming"));
  assert.equal(
    await page.locator(".stimer").count(),
    0,
    "ready/show after stop cannot restart capture",
  );
  await emit("hide-overlay", {
    captureId: "2",
    revision: "8",
    reason: "failed",
  });
  await page.waitForFunction(() => !document.querySelector(".scard"));
  await emit("show-overlay", show("2", "9"));
  assert.equal(await page.locator(".scard").count(), 0);
  await emit("recording-ready", ready("3", "10"));
  await emit("show-overlay", show("3", "10"));
  await page.locator(".stimer").waitFor();
  assert.equal(
    await page.locator(".stimer").textContent(),
    "00:00",
    "ready before show is matched by identity",
  );
});

test("late settings never revive hidden or superseded capture and rejection is handled", async (t) => {
  const { page, emit, errors } = await fixture(t);
  await page.waitForFunction(
    () => window.overlayFixture.counts().listeners.length === 6,
  );
  await page.evaluate(() => {
    window.overlayFixture.deferSettings = true;
  });
  await emit("show-overlay", show());
  await emit("recording-ready", ready());
  await page.locator(".stimer").waitFor();
  await emit("hide-overlay", {
    captureId: "1",
    revision: "2",
    reason: "cancelled",
  });
  await page.evaluate(() => window.overlayFixture.settleSettings("top"));
  await page.waitForFunction(() => !document.querySelector(".scard"));
  await emit("show-overlay", show("2", "3"));
  await emit("show-overlay", show("3", "4"));
  await emit("recording-ready", ready("3", "4"));
  await page.locator(".stimer").waitFor();
  await page.evaluate(() =>
    window.overlayFixture.settleSettings("top", false, 1),
  );
  assert.equal(
    await page.locator(".ov-stage.bottom").count(),
    1,
    "superseded settings must not change placement",
  );
  await page.evaluate(() => window.overlayFixture.settleSettings("top", true));
  assert.deepEqual(errors, []);
});

test("pending registration and settings dispose exactly once across StrictMode unmount and remount", async (t) => {
  const { page, errors } = await fixture(t, {
    pendingRegistration: true,
    strict: true,
  });
  await page.waitForFunction(
    () => window.overlayFixture.counts().pending >= 12,
  );
  await page.evaluate(() => window.overlayFixture.unmount());
  await page.evaluate(() => window.overlayFixture.settleRegistrations());
  await page.waitForFunction(
    () => window.overlayFixture.counts().listeners.length === 0,
  );
  await page.evaluate(() => window.overlayFixture.mount());
  await page.waitForFunction(
    () => window.overlayFixture.counts().pending >= 12,
  );
  await page.evaluate(() => window.overlayFixture.settleRegistrations());
  await page.waitForFunction(
    () => window.overlayFixture.counts().listeners.length === 6,
  );
  assert.equal(
    new Set(await page.evaluate(() => window.overlayFixture.counts().listeners))
      .size,
    6,
  );
  await page.evaluate(() => {
    window.overlayFixture.deferSettings = true;
    window.overlayFixture.emit("show-overlay", {
      captureId: "1",
      revision: "1",
      state: "recording",
    });
    window.overlayFixture.emit("recording-ready", {
      captureId: "1",
      captureRevision: "1",
    });
  });
  await page.locator(".stimer").waitFor();
  await page.evaluate(() => window.overlayFixture.unmount());
  await page.evaluate(() => window.overlayFixture.settleSettings());
  await page.waitForFunction(
    () =>
      window.overlayFixture.counts().listeners.length === 0 &&
      window.overlayFixture.counts().intervals === 0,
  );
  const disposed = await page.evaluate(
    () => window.overlayFixture.counts().disposed,
  );
  assert.equal(
    new Set(disposed).size,
    disposed.length,
    "each successful listener is disposed once",
  );
  assert.deepEqual(errors, []);
});

test("partially resolved and rejected registrations clean up without an unhandled rejection", async (t) => {
  const { page, errors } = await fixture(t, { pendingRegistration: true });
  await page.waitForFunction(
    () => window.overlayFixture.counts().pending === 6,
  );
  await page.evaluate(() =>
    window.overlayFixture.settleRegistrations(false, 2),
  );
  await page.evaluate(() => window.overlayFixture.settleRegistrations(true, 1));
  await page.evaluate(() => {
    window.overlayFixture.unmount();
    window.overlayFixture.emit("show-overlay", {
      captureId: "1",
      revision: "1",
      state: "recording",
    });
    window.overlayFixture.emit("recording-ready", {
      captureId: "1",
      captureRevision: "1",
    });
  });
  await page.evaluate(() => window.overlayFixture.settleRegistrations());
  await page.waitForFunction(
    () => window.overlayFixture.counts().listeners.length === 0,
  );
  assert.equal(
    await page.evaluate(() => window.overlayFixture.counts().intervals),
    0,
  );
  assert.deepEqual(errors, []);
});

test("compact processing and every terminal reason clear the timer without reviving the capture", async (t) => {
  const { page, emit, advance } = await fixture(t);
  await page.waitForFunction(
    () => window.overlayFixture.counts().listeners.length === 6,
  );
  let capture = 10;
  let revision = 100;
  for (const state of ["transcribing", "processing"]) {
    const id = String(capture++);
    const shown = String(revision++);
    await emit("show-overlay", show(id, shown));
    await emit("recording-ready", ready(id, shown));
    await page.locator(".stimer").waitFor();
    await emit("show-overlay", show(id, String(revision++), state));
    await page.locator(".sspinner").waitFor();
    await emit("recording-ready", ready(id, shown));
    assert.equal(await page.locator(".stimer").count(), 0);
  }
  for (const reason of ["closed", "finished", "cancelled", "failed"]) {
    const id = String(capture++);
    const shown = String(revision++);
    await emit("show-overlay", show(id, shown));
    await emit("recording-ready", ready(id, shown));
    await page.locator(".stimer").waitFor();
    await emit("hide-overlay", {
      captureId: id,
      revision: String(revision++),
      reason,
    });
    await page.waitForFunction(() => !document.querySelector(".scard"));
    await advance(10000);
    await emit("recording-ready", ready(id, shown));
    await emit("show-overlay", show(id, String(revision++)));
    assert.equal(await page.locator(".scard").count(), 0);
  }
  await emit("show-overlay", show("9007199254740993", "9007199254740993"));
  await emit("recording-ready", ready("9007199254740993", "9007199254740993"));
  await page.locator(".stimer").waitFor();
  await advance(6000000);
  await page.waitForFunction(
    () => document.querySelector(".stimer")?.textContent === "100:00",
  );
  await emit("hide-overlay", {
    captureId: "9007199254740992",
    revision: "9007199254740994",
    reason: "finished",
  });
  assert.equal(
    await page.locator(".stimer").textContent(),
    "100:00",
    "decimal identity preserves u64 precision",
  );
});
