import path from "node:path";
import { spawnSync } from "node:child_process";
import { PackageError } from "./manifest.mjs";

export async function inspectWindowsHost() {
  if (process.platform !== "win32" || process.arch !== "x64")
    throw new PackageError(
      "unsupported-platform",
      "This release supports Windows x64 only.",
    );
  const systemRoot = process.env.SystemRoot;
  if (!systemRoot || !path.isAbsolute(systemRoot))
    throw new PackageError(
      "host-check-failed",
      "Unable to inspect the Windows user token.",
    );
  // whoami inherits this process token. CSV contains invariant integrity SIDs,
  // irrespective of the translated group names. Never forward its output.
  const result = spawnSync(
    path.join(systemRoot, "System32/whoami.exe"),
    ["/groups", "/fo", "csv", "/nh"],
    {
      shell: false,
      windowsHide: true,
      encoding: "utf8",
      timeout: 5000,
      maxBuffer: 128 * 1024,
    },
  );
  if (result.error || result.status !== 0)
    throw new PackageError(
      "host-check-failed",
      "Unable to inspect the Windows user token.",
    );
  const levels = [...result.stdout.matchAll(/S-1-16-(\d+)/g)].map((match) =>
    Number(match[1]),
  );
  if (levels.length !== 1)
    throw new PackageError(
      "host-check-failed",
      "Unable to inspect the Windows user token.",
    );
  return { integrity: levels[0] === 8192 ? "medium" : "unsupported" };
}
