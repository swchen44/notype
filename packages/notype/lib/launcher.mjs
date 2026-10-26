import path from "node:path";
import { createRequire } from "node:module";
import { realpath } from "node:fs/promises";
import { spawn } from "node:child_process";
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
async function inspectInputs(
  {
    facadeRoot,
    platform = process.platform,
    arch = process.arch,
    env = process.env,
  },
  probe,
  strictModels = false,
) {
  if (platform !== "win32" || arch !== "x64")
    throw new PackageError(
      "unsupported-platform",
      "This release supports Windows x64 only.",
    );
  const release = await jsonFile(path.join(facadeRoot, "release.json"));
  if (release.schema !== 2 || release.status !== "assembled")
    throw new PackageError(
      "release-not-assembled",
      "This package is a packaging foundation. A verified application release has not been assembled.",
    );
  validatePin(release.application);
  const facade = await jsonFile(path.join(facadeRoot, "package.json"));
  if (facade.version !== release.version)
    throw new PackageError(
      "invalid-manifest",
      "Facade and release versions differ.",
    );
  if (Object.keys(facade.dependencies ?? {}).length !== 1 || facade.dependencies?.[release.application.name] !== release.application.version || Object.keys(facade.optionalDependencies ?? {}).length !== 0)
    throw new PackageError("invalid-manifest", "The base facade must depend only on its pinned application payload.");
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
  const application = await verifyPackage(
    applicationRoot,
    release.application,
    "application",
  );
  let modelRoot;
  let model;
  if (release.model !== undefined) {
    try {
      validatePin(release.model);
      modelRoot = packageRoot(release.model);
      model = await verifyPackage(modelRoot, release.model, "model");
    } catch (error) {
      if (strictModels) throw error;
      model = undefined;
    }
  }
  const hostProbe = path.join(applicationRoot, application.hostProbe);
  const host = probe
    ? await probe({ hostProbe, cpu: application.cpu })
    : undefined;
  const configured =
    env.NOTYPE_DATA_DIR === undefined
      ? (host?.roaming_app_data || env.APPDATA) &&
        path.join(
          host?.roaming_app_data || env.APPDATA,
          "io.github.swchen44.notype",
        )
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
  for (const root of [facadeRoot, applicationRoot, modelRoot].filter(Boolean)) {
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
      ([name]) =>
        !name.toUpperCase().startsWith("WEBVIEW2_") &&
        name.toUpperCase() !== "GGML_BACKEND_PATH" &&
        name.toUpperCase() !== "NOTYPE_DATA_DIR",
    ),
  );
  if (env.NOTYPE_DATA_DIR !== undefined)
    environment.NOTYPE_DATA_DIR = dataDirectory;
  return {
    executable: path.join(applicationRoot, application.entrypoint),
    hostProbe,
    host,
    cpu: application.cpu,
    dataDirectory,
    model: model && {
      identity: model.identity,
      root: modelRoot,
      files: model.modelFiles,
      activation: model.activation,
      artifact: model.files.find(
        (file) => file.path === model.activation.source,
      ),
    },
    environment: {
      ...environment,
      WEBVIEW2_BROWSER_EXECUTABLE_FOLDER: path.join(
        applicationRoot,
        application.webview.directory,
      ),
      WEBVIEW2_USER_DATA_FOLDER: path.join(dataDirectory, "webview"),
    },
  };
}

export async function inspectRelease(options) {
  return inspectInputs(options, undefined, true);
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
  const plan = await inspectInputs(
    options,
    typeof options.host === "function" ? options.host : () => options.host,
  );
  const host = plan.host;
  medium(host);
  if (plan.cpu === "avx2-bmi2" && (!host.cpu?.avx2 || !host.cpu?.bmi2))
    throw new PackageError(
      "unsupported-cpu",
      "This local build requires AVX2 and BMI2 CPU support. Use a compatible Windows x64 computer.",
    );
  delete plan.host;
  return plan;
}

export async function runApplication(options) {
  const plan = await prepareLaunch(options);
  if (options.signal?.aborted)
    throw new PackageError(
      "launch-cancelled",
      "Local launch was cancelled.",
    );
  if (options.launch) return options.launch(plan);
  return new Promise((resolve, reject) => {
    const child = spawn(
      plan.executable,
      options.startHidden ? ["--start-hidden"] : [],
      {
        cwd: path.dirname(plan.executable),
        env: plan.environment,
        shell: false,
        windowsHide: true,
        stdio: "ignore",
      },
    );
    child.once("error", () =>
      reject(
        new PackageError(
          "app-launch-failed",
          "Unable to start the verified local application.",
        ),
      ),
    );
    child.once("exit", (code) => {
      if (code === 0) resolve({ exitCode: 0 });
      else
        reject(
          new PackageError(
            "app-exited",
            "The local application exited before a normal shutdown. Check the local runtime package and application logs.",
          ),
        );
    });
  });
}
