import { test } from "node:test";
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import {
  mkdtemp,
  mkdir,
  writeFile,
  link,
  rm,
  readFile,
} from "node:fs/promises";
import { createHash } from "node:crypto";
import { tmpdir } from "node:os";
import { join, resolve, basename, sep } from "node:path";

const execute = promisify(execFile);

test(
  "formal recognition rejects an English-only model for a mixed Chinese profile",
  { timeout: 180000 },
  async () => {
    assert.ok(
      process.env.NOTYPE_DESKTOP_EXE,
      "Set the built formal notype executable",
    );
    assert.ok(
      process.env.NOTYPE_TINY_EN_MODEL,
      "Set the verified English-only comparison model",
    );
    assert.ok(
      process.env.NOTYPE_MIXED_WAV,
      "Set the fixed non-secret mixed-language WAV",
    );
    const model = await readFile(process.env.NOTYPE_TINY_EN_MODEL);
    assert.equal(
      model.length,
      77704715,
      "English-only negative fixture must be complete",
    );
    assert.equal(
      createHash("sha256").update(model).digest("hex"),
      "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f",
    );
    const directory = await mkdtemp(
      join(tmpdir(), "notype-recognition-receipt-"),
    );
    try {
      await mkdir(join(directory, "models"));
      // Deliberately hide the original .en filename: actual model capabilities
      // must reject this custom model even when its name claims nothing.
      await link(
        resolve(process.env.NOTYPE_TINY_EN_MODEL),
        join(directory, "models", "renamed-model.bin"),
      );
      await writeFile(
        join(directory, "settings_store.json"),
        JSON.stringify({
          settings: {
            bindings: {},
            always_on_microphone: false,
            selected_model: "renamed-model",
            recognition_mode: "traditional_english",
            app_language: "zh-TW",
            onboarding_completed: true,
          },
        }),
      );
      await assert.rejects(
        execute(
          resolve(process.env.NOTYPE_DESKTOP_EXE),
          [
            "--transcribe-file",
            resolve(process.env.NOTYPE_MIXED_WAV),
            "--model",
            "renamed-model",
            "--json",
          ],
          {
            env: { ...process.env, NOTYPE_DATA_DIR: directory },
            windowsHide: true,
            timeout: 120000,
            maxBuffer: 1024 * 1024,
          },
        ),
        (error) =>
          /recognition-model-incompatible/.test(error.stderr + error.stdout),
      );
    } finally {
      const owned = resolve(directory);
      assert.ok(
        owned.startsWith(resolve(tmpdir()) + sep) &&
          basename(owned).startsWith("notype-recognition-receipt-"),
      );
      await rm(owned, {
        recursive: true,
        force: true,
        maxRetries: 10,
        retryDelay: 250,
      });
    }
  },
);
