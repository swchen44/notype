"""Owned ZIP32 writer/extractor; all names come from a previously verified inventory."""
import hashlib
import json
import os
from pathlib import Path
import sys
import zipfile

mode, root_arg, archive_arg, inventory_arg = sys.argv[1:]
root, archive = Path(root_arg), Path(archive_arg)
entries = json.loads(Path(inventory_arg).read_text(encoding="utf-8"))
expected = {entry["path"]: entry for entry in entries}
if len(expected) != len(entries): raise ValueError('duplicate-inventory-name')

def target(name):
    parts = name.split("/")
    if not parts or not all(part and part not in (".", "..") for part in parts): raise ValueError('unsafe-path')
    if "\\" in name or ":" in name: raise ValueError('unsafe-path')
    result = root.joinpath(*parts)
    if root.resolve() not in result.resolve().parents: raise ValueError('unsafe-path')
    return result

if mode == "write":
    if not root.is_dir() or archive.exists(): raise ValueError('archive-output-invalid')
    with zipfile.ZipFile(archive, "x", compression=zipfile.ZIP_DEFLATED,
                         compresslevel=6, allowZip64=False) as output:
        for name in sorted(expected):
            source = target(name)
            if not source.is_file() or source.is_symlink(): raise ValueError('archive-input-invalid')
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.create_system = 0
            info.external_attr = 0x20
            info.compress_type = zipfile.ZIP_DEFLATED
            info._compresslevel = 6
            info.file_size = expected[name]["bytes"]
            digest = hashlib.sha256()
            length = 0
            with source.open("rb") as stream, output.open(info, "w") as destination:
                while chunk := stream.read(1024 * 1024):
                    length += len(chunk)
                    digest.update(chunk)
                    destination.write(chunk)
            if length != expected[name]["bytes"] or digest.hexdigest() != expected[name]["sha256"]: raise ValueError('archive-input-integrity')
elif mode == "extract":
    # The caller must first run the project's strict inspectZipArchive parser.
    if root.exists(): raise ValueError('extract-output-exists')
    root.mkdir()
    with zipfile.ZipFile(archive) as source:
        if len(source.namelist()) != len(expected) or set(source.namelist()) != set(expected): raise ValueError('archive-inventory-mismatch')
        for entry in source.infolist():
            if entry.is_dir() or entry.file_size != expected[entry.filename]["bytes"]: raise ValueError('archive-entry-invalid')
            destination = target(entry.filename)
            destination.parent.mkdir(parents=True, exist_ok=True)
            digest = hashlib.sha256()
            length = 0
            with source.open(entry) as stream, destination.open("xb") as output:
                while chunk := stream.read(1024 * 1024):
                    length += len(chunk)
                    if length > expected[entry.filename]["bytes"]: raise ValueError('archive-entry-too-large')
                    digest.update(chunk)
                    output.write(chunk)
            if length != expected[entry.filename]["bytes"] or digest.hexdigest() != expected[entry.filename]["sha256"]: raise ValueError('archive-entry-integrity')
else:
    raise ValueError("write or extract required")
