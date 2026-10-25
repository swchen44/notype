import assert from "node:assert/strict";
import { test } from "node:test";
import { execFileSync } from "node:child_process";

function createDiagnosticIssueUrl(preview) {
  return execFileSync(
    process.execPath,
    [
      "--experimental-strip-types",
      "--input-type=module",
      "-e",
      `import {createDiagnosticIssueUrl} from './desktop/src/lib/diagnosticIssue.ts'; process.stdout.write(createDiagnosticIssueUrl(JSON.parse(process.argv[1])));`,
      JSON.stringify(preview),
    ],
    {
      cwd: new URL("../../", import.meta.url),
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
}

test("issue draft includes only validated version and diagnostic ID, never the report", () => {
  const privateText =
    "Bearer secret https://company.invalid C:\\Users\\private\\ original speech 原稿";
  const url = new URL(
    createDiagnosticIssueUrl({
      report_id: "0123456789abcdef-00000001",
      text: `version=0.1.0\n${privateText}`,
    }),
  );
  assert.equal(
    url.origin + url.pathname,
    "https://github.com/swchen44/notype/issues/new",
  );
  assert.match(url.searchParams.get("body"), /0\.1\.0/);
  assert.match(url.searchParams.get("body"), /0123456789abcdef-00000001/);
  assert.match(url.searchParams.get("body"), /attach the exported/);
  for (const sentinel of [
    "Bearer",
    "secret",
    "company.invalid",
    "Users",
    "original speech",
    "原稿",
  ]) {
    assert.ok(!decodeURIComponent(url.href).includes(sentinel), sentinel);
  }
  const malicious = createDiagnosticIssueUrl({
    report_id: privateText,
    text: `version=${privateText}`,
  });
  for (const sentinel of [
    "Bearer",
    "secret",
    "company.invalid",
    "Users",
    "original speech",
    "原稿",
  ]) {
    assert.ok(!decodeURIComponent(malicious).includes(sentinel), sentinel);
  }
});
