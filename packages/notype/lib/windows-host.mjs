import path from "node:path";
import { spawnSync } from "node:child_process";
import { PackageError } from "./manifest.mjs";

export async function inspectWindowsHost({ hostProbe }) {
  if (process.platform !== "win32" || process.arch !== "x64")
    throw new PackageError(
      "unsupported-platform",
      "This release supports Windows x64 only.",
    );
  // The exact regular-byte helper was verified in the application manifest.
  // Its baseline Rust/Windows system APIs have no ORT or ggml imports.
  const result = spawnSync(hostProbe, [], {
    shell: false,
    windowsHide: true,
    encoding: "utf8",
    timeout: 5000,
    maxBuffer: 16 * 1024,
  });
  if (result.error || result.status !== 0)
    throw new PackageError(
      "host-check-failed",
      "Unable to verify the Windows user token and CPU compatibility.",
    );
  let report;
  try {
    report = JSON.parse(result.stdout);
  } catch {
    throw new PackageError(
      "host-check-failed",
      "The local platform probe returned invalid metadata.",
    );
  }
  if (
    report.schema !== 1 ||
    report.platform !== "win32" ||
    report.arch !== "x64" ||
    !["medium", "unsupported"].includes(report.integrity) ||
    typeof report.cpu?.avx2 !== "boolean" ||
    typeof report.cpu?.bmi2 !== "boolean" ||
    typeof report.roaming_app_data !== "string" ||
    !path.isAbsolute(report.roaming_app_data)
  )
    throw new PackageError(
      "host-check-failed",
      "The local platform probe returned invalid metadata.",
    );
  return report;
}
