#!/usr/bin/env node
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  inspectRelease,
  runApplication,
  PackageError,
} from "../lib/launcher.mjs";
import { inspectWindowsHost } from "../lib/windows-host.mjs";

const facadeRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
try {
  const args = process.argv.slice(2);
  const offlineBundle = args[0] === "--offline-bundle";
  if (offlineBundle) args.shift();
  if (
    args.length > 1 ||
    (args.length && !["--check", "--start-hidden"].includes(args[0]))
  )
    throw new PackageError(
      "invalid-arguments",
      "Usage: notype [--offline-bundle] [--check | --start-hidden]",
    );
  if (args[0] === "--check") {
    await inspectRelease({ facadeRoot, offlineBundle });
    process.stdout.write(
      "notype: package integrity verified; host compatibility and application launch were not tested.\n",
    );
  } else {
    const controller = new AbortController();
    process.once("SIGINT", () => controller.abort());
    await runApplication({
      facadeRoot,
      offlineBundle,
      host: inspectWindowsHost,
      signal: controller.signal,
      startHidden: args[0] === "--start-hidden",
    });
  }
} catch (error) {
  // Never print arbitrary filesystem/child/JSON errors or environment values.
  const known = error instanceof PackageError;
  process.stderr.write(
    `notype: ${known ? error.code : "package-check-failed"}: ${known ? error.message : "Unable to verify the local package. Reinstall the complete matching release."}\n`,
  );
  process.exitCode = 2;
}
