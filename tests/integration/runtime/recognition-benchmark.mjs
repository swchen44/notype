// Manual real-engine receipt, not a deterministic quality test or human acceptance.
// Inputs and output stay outside the repository; no API or text processor is used.
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { readFile, writeFile, mkdir, link, stat } from "node:fs/promises";
import { createHash } from "node:crypto";
import { join, resolve } from "node:path";

const execute = promisify(execFile);
const cache = resolve(process.env.NOTYPE_BENCHMARK_CACHE ?? "");
assert.ok(
  process.env.NOTYPE_BENCHMARK_CACHE,
  "Set an external benchmark cache",
);
assert.ok(process.env.NOTYPE_DESKTOP_EXE, "Set the actual formal executable");
const executable = resolve(process.env.NOTYPE_DESKTOP_EXE);
const fixtures = join(cache, "notype-stt-fixtures");
const definitions = JSON.parse(
  (await readFile(join(fixtures, "definitions.json"), "utf8")).replace(
    /^\uFEFF/,
    "",
  ),
);
const privateSenseVoice = process.argv.includes("--private-sensevoice");
const output = join(
  cache,
  privateSenseVoice
    ? "language-quality-private-sensevoice"
    : "language-quality-receipt",
);
await mkdir(output, { recursive: true });
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");
const normalized = (text) =>
  Array.from(
    text
      .normalize("NFKC")
      .toLowerCase()
      .replace(/[\p{P}\p{Z}\s]/gu, ""),
  );
function distance(left, right) {
  let row = Array.from({ length: right.length + 1 }, (_, i) => i);
  for (let i = 1; i <= left.length; i++) {
    const next = [i];
    for (let j = 1; j <= right.length; j++) {
      next[j] = Math.min(
        next[j - 1] + 1,
        row[j] + 1,
        row[j - 1] + (left[i - 1] === right[j - 1] ? 0 : 1),
      );
    }
    row = next;
  }
  return row[right.length];
}
const models = privateSenseVoice
  ? [
      {
        id: "sense-voice-int8",
        file: "model.int8.onnx",
        bytes: 239233841,
        sha256:
          "c71f0ce00bec95b07744e116345e33d8cbbe08cef896382cf907bf4b51a2cd51",
        provenance:
          "Private comparison only; publisher-to-source hashes mapped but license-owner interpretation remains unresolved; no redistribution authorization",
      },
    ]
  : [
      {
        id: "small",
        file: "ggml-small.bin",
        bytes: 487601967,
        sha256:
          "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        provenance: "Verified publisher multilingual Small FP16, MIT",
      },
      {
        id: "ggml-small-local-q5_1",
        file: "ggml-small-local-q5_1.bin",
        bytes: 190085487,
        sha256:
          "52914f6730a59593fd6108d21dcac060a35ce569d9d70eec431e5623f387c82f",
        provenance:
          "Locally quantized verified FP16 with pinned whisper.cpp v1.8.6, MIT derivative; not publisher Q5 byte identity",
      },
    ];
const receipt = {
  purpose:
    "Synthetic same-recording Chinese/English diagnostic, not human/company acceptance",
  generatedAt: new Date().toISOString(),
  executable: { path: executable, sha256: sha(await readFile(executable)) },
  options: {
    task: "transcribe",
    decodingLanguage: "auto",
    translate: false,
    manualTerms: [],
    cpuThreads: privateSenseVoice
      ? "ONNX library default, not set explicitly"
      : 4,
    requestedDevice: privateSenseVoice ? "ONNX default" : 0,
    repeatsPerLoad: 2,
  },
  metrics: {
    cer: "Levenshtein over NFKC/lowercase code points, punctuation and whitespace removed; no ITN or English repair",
    requiredLiterals:
      "Case-insensitive exact substring diagnostic, numbers may differ under inverse-text normalization",
    memory: "Not measured",
    p95: "Not estimated from this small sample",
    cold: "Fresh process/model load, OS file cache not flushed",
    warm: "Second inference in same process",
  },
  models,
  results: [],
};
for (const model of models) {
  const source = privateSenseVoice
    ? join(
        cache,
        "sensevoice-int8-provision",
        "extracted",
        model.id,
        model.file,
      )
    : join(cache, "whisper-small-provision", model.file);
  assert.equal((await stat(source)).size, model.bytes);
  assert.equal(sha(await readFile(source)), model.sha256);
  for (const mode of ["traditional_english", "simplified_english"]) {
    const directory = join(output, `${model.id}-${mode}`);
    await mkdir(join(directory, "models"), { recursive: true });
    const destination = privateSenseVoice
      ? join(directory, "models", model.id)
      : join(directory, "models");
    await mkdir(destination, { recursive: true });
    try {
      await link(source, join(destination, model.file));
    } catch (error) {
      if (error.code !== "EEXIST") throw error;
    }
    if (privateSenseVoice) {
      const tokens = join(
        cache,
        "sensevoice-int8-provision",
        "extracted",
        model.id,
        "tokens.txt",
      );
      assert.equal(
        sha(await readFile(tokens)),
        "f449eb28dc567533d7fa59be34e2abca8784f771850c78a47fb731a31429a1dc",
      );
      try {
        await link(tokens, join(destination, "tokens.txt"));
      } catch (error) {
        if (error.code !== "EEXIST") throw error;
      }
    }
    await writeFile(
      join(directory, "settings_store.json"),
      JSON.stringify({
        settings: {
          bindings: {},
          always_on_microphone: false,
          selected_model: model.id,
          recognition_mode: mode,
          app_language: "zh-TW",
          onboarding_completed: true,
          custom_words: [],
          model_unload_timeout: "never",
        },
      }),
    );
    for (const fixture of definitions.fixtures) {
      const wav = join(fixtures, `${fixture.id}.wav`);
      const wavHash = sha(await readFile(wav));
      const start = performance.now();
      const args = [
        "--transcribe-file",
        wav,
        "--model",
        model.id,
        "--repeat",
        "2",
        "--json",
      ];
      if (!privateSenseVoice) args.push("--device-index", "0");
      const { stdout, stderr } = await execute(executable, args, {
        env: { ...process.env, NOTYPE_DATA_DIR: directory },
        windowsHide: true,
        timeout: 180000,
        maxBuffer: 4 * 1024 * 1024,
      });
      await writeFile(
        join(output, `${model.id}-${mode}-${fixture.id}.log`),
        stdout + "\nSTDERR\n" + stderr,
      );
      const result = stdout
        .split(/\r?\n/)
        .reverse()
        .find(
          (line) => line.startsWith('{"') && line.includes('"transcribe_ms"'),
        );
      assert.ok(result, "Formal CLI must emit observed inference metadata");
      const observed = JSON.parse(result);
      assert.equal(observed.recognition_mode, mode);
      assert.equal(observed.cpu_threads, privateSenseVoice ? null : 4);
      const gold =
        mode === "traditional_english"
          ? fixture.goldTraditional
          : fixture.goldSimplified;
      const reference = normalized(gold);
      const errors = distance(reference, normalized(observed.text));
      const missing = fixture.mustPreserve.filter((value) =>
        /^\d+(?:\.\d+)?$/.test(value)
          ? !new RegExp(`(?<!\\d)${value.replaceAll(".", "\\.")}(?!\\d)`).test(
              observed.text,
            )
          : !observed.text.toLowerCase().includes(value.toLowerCase()),
      );
      const entry = {
        model: model.id,
        mode,
        fixture: fixture.id,
        wavSha256: wavHash,
        gold,
        observed,
        wallMs: Math.round(performance.now() - start),
        characterErrors: errors,
        referenceCharacters: reference.length,
        cer: errors / reference.length,
        missingRequiredLiterals: missing,
        strictLiteralDiagnosticPass: missing.length === 0,
      };
      receipt.results.push(entry);
      await writeFile(
        join(output, "receipt.json"),
        JSON.stringify(receipt, null, 2) + "\n",
      );
      process.stdout.write(
        `${model.id} ${mode} ${fixture.id}: CER ${(entry.cer * 100).toFixed(1)}%, missing ${missing.length}, ${observed.transcribe_ms.join("/")}ms\n`,
      );
    }
  }
}
const hardware = await execute(
  "powershell.exe",
  [
    "-NoProfile",
    "-Command",
    "Get-CimInstance Win32_Processor | Select-Object Name,NumberOfCores,NumberOfLogicalProcessors | ConvertTo-Json -Compress",
  ],
  { windowsHide: true },
);
receipt.hardware = JSON.parse(hardware.stdout);
receipt.qualityAcceptance =
  "not-established; inspect literal losses and human recordings separately";
await writeFile(
  join(output, "receipt.json"),
  JSON.stringify(receipt, null, 2) + "\n",
);
process.stdout.write(`Receipt: ${join(output, "receipt.json")}\n`);
