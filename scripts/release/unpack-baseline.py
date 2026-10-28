"""Unpack the independently pinned local.7 baseline into a new owned directory."""
import argparse
import hashlib
import os
from pathlib import Path
import re
import shutil
import stat
import zipfile

PIN = 'b224031c3d05a2859beaadc519b8d0f9239600fefb88721b929c684c0ceb5b9b'
SIZE = 1054038702
EXPANDED = 1583274510
COUNT = 1733

def run(archive, destination):
    archive = Path(archive)
    destination = Path(destination)
    if destination.exists() or not archive.is_file() or archive.stat().st_size != SIZE:
        raise ValueError('baseline-input-invalid')
    digest = hashlib.sha256()
    with archive.open('rb') as stream:
        while chunk := stream.read(1024 * 1024): digest.update(chunk)
    if digest.hexdigest() != PIN: raise ValueError('baseline-archive-hash')
    with zipfile.ZipFile(archive) as bundle:
        entries = bundle.infolist()
        if len(entries) != COUNT or sum(x.file_size for x in entries) != EXPANDED:
            raise ValueError('baseline-inventory')
        seen = set()
        for item in entries:
            parts = item.filename.split('/')
            if (not parts or any(not p or p in ('.','..') or re.search(r'[\\:*?"<>|\x00-\x1f]',p) or p.endswith(('.', ' ')) or re.match(r'^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)',p,re.I) for p in parts)
                or item.is_dir() or item.flag_bits & 1 or item.filename.casefold() in seen
                or (item.create_system == 3 and stat.S_ISLNK(item.external_attr >> 16))):
                raise ValueError('unsafe-baseline-entry')
            seen.add(item.filename.casefold())
        destination.mkdir(parents=True)
        for item in entries:
            target = destination.joinpath(*item.filename.split('/'))
            target.parent.mkdir(parents=True, exist_ok=True)
            with bundle.open(item) as src, target.open('xb') as dst:
                shutil.copyfileobj(src,dst,1024 * 1024)

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--archive',required=True)
    parser.add_argument('--destination',required=True)
    args = parser.parse_args()
    run(args.archive,args.destination)
