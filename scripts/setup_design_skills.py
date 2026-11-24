"""Operator-run setup for the two reviewed user-supplied ZIPs. Never runs at startup."""
import argparse
import hashlib
import os
from pathlib import Path, PurePosixPath
import stat
import subprocess
import sys
import zipfile

ARCHIVES = {
    'colour-intelligence': '7620aba647a80f2085fe41f859aadb2061c5e816f071cde9989ae24c8f6238ee',
    'professional-document-design': '5ddd40915c7b2b3b4700f5f40f17dbcff9234d8bbacf0402db5db6281fd31f67',
}


DOCUMENT_V2 = {'professional-document-design-v2': 'a83e33b35a0ba7d30b6099fb13078f141d58e31a9439c17cfe9ec4dbe8744e28'}
COLOUR_V2 = {'colour-intelligence-v2': '20d4fad76e54bfb54643fb08c2273fb9b0ba544eb9da757a927f436ebc4ff381'}


def unpack(archives, destination, manifest=None):
    for name, expected in (ARCHIVES if manifest is None else manifest).items():
        archive = archives / (name + '.zip')
        if hashlib.sha256(archive.read_bytes()).hexdigest() != expected:
            raise ValueError('Archive differs from reviewed version: ' + name)
        with zipfile.ZipFile(archive) as source:
            entries = source.infolist()
            if len(entries) > 128 or sum(item.file_size for item in entries) > 4 * 1024 * 1024:
                raise ValueError('Archive exceeds limits')
            seen = set()
            for item in entries:
                relative = PurePosixPath(item.filename)
                if not relative.parts or relative.parts[0] != name or relative.is_absolute() or '..' in relative.parts or '\\' in item.filename or ':' in item.filename or stat.S_ISLNK(item.external_attr >> 16) or item.file_size > 512 * 1024 or item.filename.lower() in seen:
                    raise ValueError('Unsafe or duplicate archive entry')
                seen.add(item.filename.lower())
                target = destination.joinpath(*relative.parts)
                for parent in [target, *target.parents]:
                    if parent.exists() and (parent.is_symlink() or (hasattr(parent, 'is_junction') and parent.is_junction())):
                        raise ValueError('Linked installation paths are unsupported')
                if item.is_dir():
                    target.mkdir(parents=True, exist_ok=True)
                else:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    data = source.read(item)
                    if target.exists():
                        if target.read_bytes() != data:
                            raise ValueError('Existing installed package differs; choose a new profile')
                    else:
                        with target.open('xb') as output:
                            output.write(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archives', type=Path, required=True)
    parser.add_argument('--agent', type=Path, required=True, help='vmt-agent-check executable')
    parser.add_argument('--soffice', type=Path, required=True)
    parser.add_argument('--profile', type=Path, default=Path(os.environ['LOCALAPPDATA']) / 'VMT/LocalAgent')
    parser.add_argument('--install-dependencies', action='store_true', help='Explicitly permit pip downloads into the private venv')
    parser.add_argument('--colour-v2', action='store_true', help='Install the reviewed advanced colour package alongside V1')
    parser.add_argument('--document-v2', action='store_true', help='Install the reviewed V2 document package alongside V1')
    args = parser.parse_args()
    profile = args.profile.absolute()
    packages = profile / 'skill-packages/design-2026-11-24'
    unpack(args.archives, packages)
    if args.colour_v2:
        unpack(args.archives, packages, COLOUR_V2)
    if args.document_v2:
        unpack(args.archives, packages, DOCUMENT_V2)
    environment = profile / 'design-runtime/venv'
    python = environment / ('Scripts/python.exe' if os.name == 'nt' else 'bin/python')
    if args.install_dependencies:
        subprocess.run([sys.executable, '-m', 'venv', str(environment)], check=True)
        subprocess.run([str(python), '-m', 'pip', 'install', '--only-binary=:all:', '-r', str(Path(__file__).with_name('design-requirements.txt'))], check=True)
    if not python.is_file():
        raise ValueError('Private Python runtime missing; rerun with --install-dependencies')
    subprocess.run([str(python), '-I', '-c', 'import PIL, docx, pptx, fitz'], check=True)
    agent = str(args.agent.resolve())
    subprocess.run([agent, '--skills-import', str(packages), '--data-dir', str(profile)], check=True)
    subprocess.run([agent, '--design-configure', str(python), str(args.soffice.resolve()), '--data-dir', str(profile)], check=True)
    print('Setup complete. Skill registration does not grant execution permission.')


if __name__ == '__main__':
    main()
