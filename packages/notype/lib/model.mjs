import {
  mkdir,
  lstat,
  open,
  link,
  unlink,
  readdir,
  statfs,
} from "node:fs/promises";
import { createReadStream } from "node:fs";
import { createHash, randomUUID } from "node:crypto";
import path from "node:path";
import { PackageError, sha256 } from "./manifest.mjs";

async function existing(file, artifact) {
  let stat;
  try {
    stat = await lstat(file);
  } catch (error) {
    if (error.code === "ENOENT") return false;
    throw error;
  }
  if (
    stat.isSymbolicLink() ||
    !stat.isFile() ||
    stat.size !== artifact.bytes ||
    (await sha256(file)) !== artifact.sha256
  )
    throw new PackageError(
      "model-conflict",
      "An existing model differs from the pinned model. It has been retained; select another data directory or resolve the model conflict explicitly.",
    );
  return true;
}

export async function activateModel(
  plan,
  { signal, onProgress, minimumFreeBytes = 256 * 1024 * 1024 } = {},
) {
  const cancelled = () => {
    if (signal?.aborted)
      throw new PackageError(
        "model-activation-cancelled",
        "Local model activation was cancelled; existing models and history have been retained.",
      );
  };
  cancelled();
  const { root, activation, artifact } = plan.model;
  const directory = path.join(plan.dataDirectory, "models");
  await mkdir(directory, { recursive: true });
  const status = await lstat(directory);
  if (!status.isDirectory() || status.isSymbolicLink())
    throw new PackageError(
      "unsafe-model-directory",
      "The model directory must be a regular local directory.",
    );
  // Only this implementation's exact reserved names for this pinned content
  // are eligible. Never touch a live/unknown process, links or unrelated files.
  const partialPattern = new RegExp(
    `^\\.notype-${artifact.sha256}-(\\d+)-[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}\\.partial$`,
  );
  for (const name of await readdir(directory)) {
    const match = partialPattern.exec(name);
    if (!match || !Number.isSafeInteger(Number(match[1]))) continue;
    try {
      process.kill(Number(match[1]), 0);
      continue;
    } catch (error) {
      if (error.code !== "ESRCH") continue;
    }
    const candidate = path.join(directory, name);
    const metadata = await lstat(candidate).catch(() => undefined);
    if (
      metadata?.isFile() &&
      !metadata.isSymbolicLink() &&
      metadata.size <= artifact.bytes
    )
      await unlink(candidate).catch((error) => {
        if (error.code !== "ENOENT") throw error;
      });
  }
  const destination = path.join(directory, activation.filename);
  const result = (status) => ({
    status,
    path: destination,
    modelId: activation.modelId,
  });
  if (await existing(destination, artifact)) {
    cancelled();
    return result("existing");
  }
  if (!Number.isSafeInteger(minimumFreeBytes) || minimumFreeBytes < 0)
    throw new PackageError(
      "invalid-activation-options",
      "Invalid model activation storage margin.",
    );
  const volume = await statfs(directory, { bigint: true });
  if (
    volume.bavail * volume.bsize <
    BigInt(artifact.bytes) + BigInt(minimumFreeBytes)
  )
    throw new PackageError(
      "insufficient-model-space",
      "Not enough local disk space to activate the verified model with a working margin. Existing data has been retained.",
    );
  const source = path.join(root, activation.source);
  const sourceMetadata = await lstat(source);
  if (
    !sourceMetadata.isFile() ||
    sourceMetadata.isSymbolicLink() ||
    sourceMetadata.size !== artifact.bytes
  )
    throw new PackageError(
      "model-source-changed",
      "The installed model changed before activation. Reinstall the matching package.",
    );
  const temporary = path.join(
    directory,
    `.notype-${artifact.sha256}-${process.pid}-${randomUUID()}.partial`,
  );
  let handle;
  try {
    handle = await open(temporary, "wx");
    const hash = createHash("sha256");
    let bytes = 0;
    for await (const chunk of createReadStream(source)) {
      cancelled();
      hash.update(chunk);
      bytes += chunk.length;
      if (bytes > artifact.bytes)
        throw new PackageError(
          "model-source-changed",
          "The installed model grew during activation. Reinstall the matching package.",
        );
      let written = 0;
      while (written < chunk.length)
        written += (await handle.write(chunk, written, chunk.length - written))
          .bytesWritten;
      onProgress?.({ bytes, total: artifact.bytes });
    }
    cancelled();
    if (bytes !== artifact.bytes || hash.digest("hex") !== artifact.sha256)
      throw new PackageError(
        "model-source-changed",
        "The installed model changed during activation. Reinstall the matching package.",
      );
    await handle.sync();
    await handle.close();
    handle = undefined;
    cancelled();
    // An exclusive hard-link publication is atomic and never replaces a file
    // another instance/user created meanwhile. No partial final model is visible.
    try {
      await link(temporary, destination);
    } catch (error) {
      if (error.code === "EEXIST" && (await existing(destination, artifact)))
        return result("existing");
      throw new PackageError(
        "model-activation-failed",
        "Unable to atomically activate the local model. Use a writable local filesystem with hard-link support.",
      );
    }
    cancelled();
    return result("activated");
  } finally {
    if (handle) await handle.close();
    try {
      await unlink(temporary);
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
  }
}
