#!/usr/bin/env node
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  inspectRelease,
  prepareLaunch,
  PackageError,
} from "../lib/launcher.mjs";
import { inspectWindowsHost } from "../lib/windows-host.mjs";

const facadeRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
try {
  const args = process.argv.slice(2);
  if (args.length > 1 || (args.length && args[0] !== "--check"))
    throw new PackageError("invalid-arguments", "Usage: notype [--check]");
  if (args[0] === "--check") {
    await inspectRelease({ facadeRoot });
    process.stdout.write(
      "notype: package integrity verified; host compatibility and application launch were not tested.\n",
    );
  } else {
    await prepareLaunch({ facadeRoot, host: inspectWindowsHost });
    // Native/model activation is intentionally not wired until the final #13
    // binary and shipping model have been assembled and verified in #14.
    throw new PackageError(
      "launch-not-integrated",
      "Package checks passed, but local model activation and the release application have not been integrated.",
    );
  }
} catch (error) {
  // Never print arbitrary filesystem/child/JSON errors or environment values.
  const known = error instanceof PackageError;
  process.stderr.write(
    `notype: ${known ? error.code : "package-check-failed"}: ${known ? error.message : "Unable to verify the local package. Reinstall the complete matching release."}\n`,
  );
  process.exitCode = 2;
}
