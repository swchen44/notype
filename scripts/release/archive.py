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
assert len(expected) == len(entries)

def target(name):
    parts = name.split("/")
    assert parts and all(part and part not in (".", "..") for part in parts)
    assert "\\" not in name and ":" not in name
    result = root.joinpath(*parts)
    assert root.resolve() in result.resolve().parents
    return result

if mode == "write":
    assert root.is_dir() and not archive.exists()
    with zipfile.ZipFile(archive, "x", compression=zipfile.ZIP_DEFLATED,
                         compresslevel=6, allowZip64=False) as output:
        for name in sorted(expected):
            source = target(name)
            assert source.is_file() and not source.is_symlink()
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
            assert length == expected[name]["bytes"]
            assert digest.hexdigest() == expected[name]["sha256"]
elif mode == "extract":
    # The caller must first run the project's strict inspectZipArchive parser.
    assert not root.exists()
    root.mkdir()
    with zipfile.ZipFile(archive) as source:
        assert set(source.namelist()) == set(expected)
        for entry in source.infolist():
            assert not entry.is_dir() and entry.file_size == expected[entry.filename]["bytes"]
            destination = target(entry.filename)
            destination.parent.mkdir(parents=True, exist_ok=True)
            digest = hashlib.sha256()
            length = 0
            with source.open(entry) as stream, destination.open("xb") as output:
                while chunk := stream.read(1024 * 1024):
                    length += len(chunk)
                    assert length <= expected[entry.filename]["bytes"]
                    digest.update(chunk)
                    output.write(chunk)
            assert length == expected[entry.filename]["bytes"]
            assert digest.hexdigest() == expected[entry.filename]["sha256"]
else:
    raise ValueError("write or extract required")
