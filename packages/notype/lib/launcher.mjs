import path from "node:path";
import { createRequire } from "node:module";
import { realpath } from "node:fs/promises";
import {
  PackageError,
  jsonFile,
  verifyPackage,
  validatePin,
} from "./manifest.mjs";
export { PackageError } from "./manifest.mjs";

function medium(host) {
  if (host?.integrity !== "medium")
    throw new PackageError(
      "normal-user-required",
      "Run notype from a normal Windows user terminal, without administrator elevation.",
    );
}
function contains(parent, child) {
  const relative = path.relative(parent, child);
  return (
    relative === "" ||
    (!relative.startsWith(`..${path.sep}`) &&
      relative !== ".." &&
      !path.isAbsolute(relative))
  );
}
async function resolvedFuturePath(directory) {
  let existing = directory;
  const missing = [];
  for (;;) {
    try {
      return path.join(await realpath(existing), ...missing.reverse());
    } catch (error) {
      if (error.code !== "ENOENT")
        throw new PackageError(
          "invalid-data-directory",
          "Unable to resolve a stable local data directory.",
        );
      missing.push(path.basename(existing));
      const parent = path.dirname(existing);
      if (parent === existing)
        throw new PackageError(
          "invalid-data-directory",
          "Unable to resolve a stable local data directory.",
        );
      existing = parent;
    }
  }
}
export async function inspectRelease({
  facadeRoot,
  platform = process.platform,
  arch = process.arch,
  env = process.env,
}) {
  if (platform !== "win32" || arch !== "x64")
    throw new PackageError(
      "unsupported-platform",
      "This release supports Windows x64 only.",
    );
  const release = await jsonFile(path.join(facadeRoot, "release.json"));
  if (release.schema !== 1 || release.status !== "assembled")
    throw new PackageError(
      "release-not-assembled",
      "This package is a packaging foundation. A verified application and model release has not been assembled.",
    );
  validatePin(release.application);
  validatePin(release.model);
  const facade = await jsonFile(path.join(facadeRoot, "package.json"));
  if (facade.version !== release.version)
    throw new PackageError(
      "invalid-manifest",
      "Facade and release versions differ.",
    );
  const require = createRequire(path.join(facadeRoot, "package.json"));
  function packageRoot(pin) {
    try {
      return path.dirname(require.resolve(`${pin.name}/package.json`));
    } catch {
      throw new PackageError(
        "payload-missing",
        "A pinned notype dependency is missing. Install the complete matching release; no download will be attempted at launch.",
      );
    }
  }
  const applicationRoot = packageRoot(release.application);
  const modelRoot = packageRoot(release.model);
  const application = await verifyPackage(
    applicationRoot,
    release.application,
    "application",
  );
  const model = await verifyPackage(modelRoot, release.model, "model");
  const configured =
    env.NOTYPE_DATA_DIR === undefined
      ? env.APPDATA && path.join(env.APPDATA, "io.github.swchen44.notype")
      : env.NOTYPE_DATA_DIR;
  if (
    typeof configured !== "string" ||
    !path.isAbsolute(configured) ||
    (process.platform === "win32" && !/^[a-z]:[\\/]/i.test(configured)) ||
    /^[\\/]{2}/.test(configured) ||
    /[\x00-\x1f]/.test(configured)
  )
    throw new PackageError(
      "invalid-data-directory",
      "Use an absolute local NOTYPE_DATA_DIR outside the installed packages, or provide the Windows roaming application-data directory.",
    );
  const dataDirectory = path.resolve(configured);
  const resolvedData = await resolvedFuturePath(dataDirectory);
  for (const root of [facadeRoot, applicationRoot, modelRoot]) {
    const packageDirectory = await realpath(root);
    if (
      contains(packageDirectory, resolvedData) ||
      contains(resolvedData, packageDirectory)
    )
      throw new PackageError(
        "invalid-data-directory",
        "Persistent data must be separate from installed package files.",
      );
  }
  const environment = Object.fromEntries(
    Object.entries(env).filter(
      ([name]) => !name.toUpperCase().startsWith("WEBVIEW2_"),
    ),
  );
  return {
    executable: path.join(applicationRoot, application.entrypoint),
    dataDirectory,
    model: {
      identity: model.identity,
      root: modelRoot,
      files: model.modelFiles,
    },
    environment: {
      ...environment,
      NOTYPE_DATA_DIR: dataDirectory,
      WEBVIEW2_BROWSER_EXECUTABLE_FOLDER: path.join(
        applicationRoot,
        application.webview.directory,
      ),
      WEBVIEW2_USER_DATA_FOLDER: path.join(dataDirectory, "webview2"),
    },
  };
}

export async function prepareLaunch(options) {
  if (
    (options.platform ?? process.platform) !== "win32" ||
    (options.arch ?? process.arch) !== "x64"
  )
    throw new PackageError(
      "unsupported-platform",
      "This release supports Windows x64 only.",
    );
  // A supplied host object is useful to platform adapters; production supplies a
  // token probe function, invoked only after missing/corrupt packages are refused.
  if (typeof options.host !== "function") medium(options.host);
  const plan = await inspectRelease(options);
  if (typeof options.host === "function") medium(await options.host());
  return plan;
}
