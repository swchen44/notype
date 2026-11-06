"""Repack the already compiled, pinned model-free portable release as 7z.

Never modifies the original release. Verifies every extracted file before upload.
"""
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile
import zipfile

ZIP_SHA = '283a550be00fbab7c1abc928e6230e7e290f8ae972d17599d15de2a6f8271d10'
EXE_SHA = 'c86e6d3a82529ae46be679bb98a8875831aa558788bbce991d71a41d0f58b3dc'
SOURCE = 'bc20d5eb512439c68aefe65b8c9985f106b9768c'

def digest(file):
    with open(file, 'rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

def inventory(root):
    return {p.relative_to(root).as_posix(): (p.stat().st_size, digest(p))
            for p in root.rglob('*') if p.is_file()}

def repack(archive, output, seven):
    if digest(archive) != ZIP_SHA:
        raise ValueError('source ZIP checksum mismatch')
    output.mkdir()  # Do not overwrite another output.
    with tempfile.TemporaryDirectory(prefix='notype-7z-', dir=output.parent) as temporary:
        work = pathlib.Path(temporary)
        original, extracted = work / 'original', work / 'extracted'
        with zipfile.ZipFile(archive) as source:
            entries = source.infolist()
            if len(entries) != 1723 or sum(i.file_size for i in entries) != 892323350:
                raise ValueError('source ZIP inventory changed')
            for entry in entries:
                p = pathlib.PurePosixPath(entry.filename)
                if p.is_absolute() or '..' in p.parts or ':' in entry.filename or '\\' in entry.filename:
                    raise ValueError('unsafe archive entry')
                if (entry.external_attr >> 16) & 0o170000 == 0o120000:
                    raise ValueError('archive symlink rejected')
            source.extractall(original)
        before = inventory(original)
        executables = list(original.rglob('notype.exe'))
        if len(executables) != 1 or digest(executables[0]) != EXE_SHA:
            raise ValueError('compiled application mismatch')
        name = 'notype-0.1.2-local.1-portable-win-x64.7z'
        destination = output / name
        def run(args, cwd=None):
            subprocess.run([seven, *args], cwd=cwd, check=True, timeout=1800, capture_output=True)
        run(['a', '-t7z', '-mx=5', '-mmt=2', str(destination), '.'], original)
        run(['t', str(destination)])
        run(['x', str(destination), '-o' + str(extracted), '-y'])
        if inventory(extracted) != before:
            raise ValueError('7z extracted file mismatch')
        sha = digest(destination)
        (output / (name + '.sha256')).write_text(sha + '  ' + name + '\n', encoding='utf-8')
        (output / 'PORTABLE-7Z-VERIFICATION.json').write_text(json.dumps({
            'source': SOURCE, 'source_zip_sha256': ZIP_SHA, 'exe_sha256': EXE_SHA,
            'archive': name, 'sha256': sha, 'bytes': destination.stat().st_size,
            'files': len(before), 'checks': ['source-zip-pin', 'compiled-exe-pin', '7z-test', 'fresh-extraction-all-file-sha256'],
            'models_included': False, 'note': 'Same compiled application as the verified ZIP; no recompilation. Full GUI, microphone and company network acceptance remain manual.'
        }, indent=2) + '\n', encoding='utf-8')

if __name__ == '__main__':
    repack(pathlib.Path(sys.argv[1]).resolve(), pathlib.Path(sys.argv[2]).resolve(), sys.argv[3])
