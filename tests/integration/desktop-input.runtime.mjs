import { test } from "node:test";
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, rm, writeFile, lstat, realpath } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { once } from "node:events";
import { createInterface } from "node:readline";
import { setTimeout as delay } from "node:timers/promises";
import { desktopFixture, invoke } from "./desktop-driver.mjs";

async function ownedControls() {
  const directory = await mkdtemp(join(tmpdir(), "notype-owned-controls-"));
  const executable = join(directory, "NativeInputFixture.exe");
  execFileSync(
    "C:/Windows/Microsoft.NET/Framework64/v4.0.30319/csc.exe",
    [
      "/nologo",
      "/target:exe",
      `/out:${executable}`,
      "/r:System.Windows.Forms.dll",
      "/r:System.Drawing.dll",
      "/r:System.Web.Extensions.dll",
      "/r:C:\\Windows\\Microsoft.NET\\Framework64\\v4.0.30319\\WPF\\UIAutomationClient.dll",
      "/r:C:\\Windows\\Microsoft.NET\\Framework64\\v4.0.30319\\WPF\\UIAutomationTypes.dll",
      "/r:C:\\Windows\\Microsoft.NET\\Framework64\\v4.0.30319\\WPF\\WindowsBase.dll",
      resolve("tests/integration/fixtures/NativeInputFixture.cs"),
    ],
    { windowsHide: true, encoding: "utf8" },
  );
  const child = spawn(executable, [], {
    windowsHide: true,
    stdio: ["pipe", "pipe", "pipe"],
  });
  const lines = createInterface({ input: child.stdout })[
    Symbol.asyncIterator
  ]();
  async function response() {
    const result = await Promise.race([
      lines.next(),
      delay(5000).then(() => {
        throw Error("Owned control fixture timeout");
      }),
    ]);
    assert.equal(result.done, false);
    const value = JSON.parse(result.value);
    assert.equal(value.error, undefined);
    return value;
  }
  const ready = await response();
  assert.equal(ready.ready, true);
  assert.equal(
    ready.pid,
    child.pid,
    "Only the child spawned by this harness is owned",
  );
  const fixtureImage = await realpath(executable);
  const imageStat = await lstat(fixtureImage);
  assert.ok(
    imageStat.isFile() && !imageStat.isSymbolicLink(),
    "Owned fixture image is regular",
  );
  // UIA SetFocus targets only the verified fixture HWND/PID/image. Production never focuses targets.
  if (process.env.NOTYPE_MANUAL_FIXTURE_FOCUS !== "1")
    execFileSync(
      executable,
      ["focus-owned", String(ready.pid), String(ready.focusHandle)],
      { windowsHide: true, timeout: 5000 },
    );
  async function command(command, args = {}) {
    child.stdin.write(JSON.stringify({ command, ...args }) + "\n");
    const state = await response();
    await delay(150); // Drain Windows accessibility focus events; not a chord deadline.
    return state;
  }
  return {
    owner: { pid: child.pid, image: fixtureImage },
    command,
    async close() {
      if (child.exitCode === null) {
        child.stdin.end();
        await once(child, "exit");
      }
      await rm(directory, {
        recursive: true,
        force: true,
        maxRetries: 8,
        retryDelay: 150,
      });
    },
  };
}

test(
  "native original-control delivery preserves selection, rejects changed/unsupported targets and never resends uncertain insertion",
  {
    timeout: process.env.NOTYPE_MANUAL_FIXTURE_FOCUS === "1" ? 360000 : 120000,
  },
  async () => {
    const desktop = await desktopFixture();
    let app, controls;
    try {
      app = await desktop.launch();
      controls = await ownedControls();
      const reset = () => controls.command("reset");
      const captureReceipts = [];
      async function begin(requireInputTarget = true) {
        const state = await controls.command("state");
        const observation = {
          case: captureReceipts.length + 1,
          foregroundOwned: state.foregroundOwned,
          requireInputTarget,
          capturedRequiredTarget: false,
        };
        captureReceipts.push(observation);
        const receiptPath = join(
          process.env.NOTYPE_DATA_DIR,
          "owned-captures.json",
        );
        await writeFile(receiptPath, JSON.stringify(captureReceipts, null, 2));
        assert.equal(
          state.foregroundOwned,
          true,
          "Owned fixture must remain foreground before every capture",
        );
        const generation = await invoke(app.page, "debug_begin_session", {
          recognitionMode: "traditional_english",
          scenario: "dictation",
          captureInput: true,
          fixtureOwner: controls.owner,
          requireInputTarget,
        });
        // With requireInputTarget=true, success is the native command's receipt
        // that it captured an owned token; no invented target or fallback exists.
        observation.capturedRequiredTarget = requireInputTarget;
        await writeFile(receiptPath, JSON.stringify(captureReceipts, null, 2));
        return generation;
      }
      async function finish(generation, transcript = "[notype Ω 🧪]") {
        const entry = await invoke(app.page, "debug_finish_session", {
          generation,
          transcript,
        });
        return invoke(app.page, "choose_history_original", { id: entry.id });
      }
      const deliver = (entry) =>
        invoke(app.page, "deliver_history_entry", { id: entry.id });
      let initial = await reset();
      if (process.env.NOTYPE_MANUAL_FIXTURE_FOCUS === "1") {
        const deadline = Date.now() + 300000;
        const receipt = join(process.env.NOTYPE_DATA_DIR, "manual-focus.json");
        while (!initial.foregroundOwned && Date.now() < deadline) {
          await writeFile(
            receipt,
            JSON.stringify({
              ready: true,
              title: "notype OWNED input fixture",
              visibleOwned: initial.visibleOwned,
              foregroundOwned: false,
              deadline: new Date(deadline).toISOString(),
              updatedAt: new Date().toISOString(),
            }),
          );
          await delay(500);
          initial = await controls.command("state");
        }
        await writeFile(
          receipt,
          JSON.stringify({
            ready: true,
            title: "notype OWNED input fixture",
            visibleOwned: initial.visibleOwned,
            foregroundOwned: initial.foregroundOwned,
            deadline: new Date(deadline).toISOString(),
            updatedAt: new Date().toISOString(),
          }),
        );
        if (initial.foregroundOwned) initial = await reset();
      }
      assert.equal(
        initial.foregroundOwned,
        true,
        `owned fixture requires foreground: ${JSON.stringify(initial)}`,
      );
      assert.equal(initial.focusFirst, true, "owned Edit has focus");
      let entry = await finish(await begin());
      entry = await deliver(entry);
      assert.equal(entry.status, "delivered", JSON.stringify(entry));
      let state = await controls.command("state");
      assert.equal(state.controls.first.text, "alpha [notype Ω 🧪] gamma");
      assert.equal(state.controls.second.text, "alpha beta gamma");
      assert.equal(state.enters, 0);
      assert.equal((await deliver(entry)).status, "delivered");
      assert.equal(
        (await controls.command("state")).controls.first.text,
        state.controls.first.text,
      );

      await reset();
      await controls.command("focus", { id: "rich" });
      entry = await finish(await begin(), "[rich]");
      assert.equal((await deliver(entry)).status, "delivered");
      assert.equal(
        (await controls.command("state")).controls.rich.text,
        "alpha [rich] gamma",
      );

      for (const returnToOriginal of [false, true]) {
        await reset();
        const generation = await begin();
        await controls.command("focus", { id: "second" });
        if (returnToOriginal) await controls.command("focus", { id: "first" });
        entry = await deliver(await finish(generation));
        assert.equal(entry.status, "pending");
        assert.equal(entry.error, "target-changed");
        state = await controls.command("state");
        assert.equal(state.controls.first.text, "alpha beta gamma");
        assert.equal(state.controls.second.text, "alpha beta gamma");
      }
      await reset();
      const selected = await begin();
      await controls.command("select", { start: 0, end: 5 });
      assert.equal(
        (await deliver(await finish(selected))).error,
        "target-changed",
      );

      for (const id of ["readonly", "password", "unsupported"]) {
        await reset();
        await controls.command("focus", { id });
        entry = await deliver(await finish(await begin(false)));
        assert.equal(entry.status, "pending", id);
        state = await controls.command("state");
        assert.equal(state.controls.first.text, "alpha beta gamma");
        if (id !== "unsupported")
          assert.equal(state.controls[id].text, "alpha beta gamma");
      }
      await reset();
      await controls.command("limit", { value: 20 });
      entry = await deliver(
        await finish(await begin(), "[replacement exceeds limit]"),
      );
      assert.equal(entry.status, "pending");
      assert.equal(
        (await controls.command("state")).controls.first.text,
        "alpha beta gamma",
      );

      await reset();
      const slow = await begin();
      await controls.command("slow", { value: true });
      entry = await deliver(await finish(slow, "[slow]"));
      assert.equal(entry.status, "delivery-unknown");
      assert.equal((await deliver(entry)).status, "delivery-unknown");
      state = await controls.command("state");
      assert.equal(state.controls.first.text, "alpha [slow] gamma");
      assert.equal(state.enters, 0);
    } finally {
      if (controls) await controls.close();
      if (app) await desktop.close(app);
      await desktop.dispose();
    }
  },
);
