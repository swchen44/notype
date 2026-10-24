"""Copy selected packaged sources as inert bytes; never execute the app or JS."""

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import struct


def leaves(files, prefix=""):
    for name, entry in files.items():
        relative = prefix + name
        if "files" in entry:
            yield from leaves(entry["files"], relative + "/")
        else:
            yield relative, entry


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("app", type=Path, help="Typeless.app path")
    parser.add_argument("output", type=Path, help="New directory outside the app")
    args = parser.parse_args()
    app = args.app.resolve()
    output = args.output.resolve()
    if output == app or app in output.parents:
        parser.error("Output must be outside the application bundle")
    if output.exists():
        parser.error("Output must be a new directory")
    archive = app / "Contents/Resources/app.asar"
    with archive.open("rb") as stream:
        prefix = stream.read(16)
        if len(prefix) != 16:
            raise ValueError("ASAR prefix is incomplete")
        header_bytes = struct.unpack("<I", prefix[12:16])[0]
        if header_bytes > 16 * 1024 * 1024:
            raise ValueError("ASAR header exceeds research reader limit")
        tree = json.loads(stream.read(header_bytes))
        base = 8 + struct.unpack("<I", prefix[4:8])[0]
        selected = []
        output.mkdir(parents=True)
        for relative, entry in leaves(tree["files"]):
            if not (relative == "package.json" or relative.startswith("dist/") and relative.endswith((".js", ".mjs", ".html"))):
                continue
            pure = PurePosixPath(relative)
            if pure.is_absolute() or ".." in pure.parts or "link" in entry:
                raise ValueError("Unsafe or linked entry")
            if entry.get("unpacked") or "offset" not in entry:
                continue
            size = entry["size"]
            start = base + int(entry["offset"])
            if not 0 <= size <= 32 * 1024 * 1024 or start < base or start + size > archive.stat().st_size:
                raise ValueError("Invalid or excessive source range")
            stream.seek(start)
            content = stream.read(size)
            destination = output / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(content)
            selected.append({"path": relative, "bytes": size, "sha256": hashlib.sha256(content).hexdigest(), "offset_in_asar": start})
    with archive.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    (output / "source-identities.json").write_text(json.dumps({"asar_sha256": digest, "files": selected}, indent=2) + "\n")
    print(json.dumps({"asar_sha256": digest, "copied_source_files": len(selected), "output": str(output)}))


if __name__ == "__main__":
    main()
