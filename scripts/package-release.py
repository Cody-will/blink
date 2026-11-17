#!/usr/bin/env python3
"""Package the native Linux x86_64 release; never publish or modify user config."""
import hashlib
import json
import platform
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True)


def main():
    if platform.system() != 'Linux' or platform.machine() != 'x86_64':
        raise SystemExit('This packaging script currently supports native Linux x86_64 only.')
    package = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']
    version = package['version']
    binary = ROOT / 'target/release/blink'
    if command(str(binary), '--version').strip() != f'blink {version}':
        raise SystemExit('Build the current release version before packaging.')
    for file in ['LICENSE', 'ARTWORK-LICENSE.md']:
        if not (ROOT / file).is_file():
            raise SystemExit(f'Missing {file}; finalize licensing before packaging.')
    metadata = json.loads(command('cargo', 'metadata', '--locked', '--format-version', '1',
                                  '--filter-platform', 'x86_64-unknown-linux-gnu'))
    name = f'blink-v{version}-x86_64-unknown-linux-gnu'
    dist = ROOT / 'dist'
    dist.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='blink-package-') as tmp:
        stage = Path(tmp) / name
        stage.mkdir()
        shutil.copy2(binary, stage / 'blink')
        for file in ['LICENSE', 'ARTWORK-LICENSE.md', 'README.md', 'CHANGELOG.md']:
            shutil.copy2(ROOT / file, stage / file)
        for directory in ['docs', 'examples', 'screenshots']:
            shutil.copytree(ROOT / directory, stage / directory)
        notices = stage / 'third-party'
        notices.mkdir()
        inventory = []
        for dep in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
            if dep['id'] in metadata['workspace_members']:
                continue
            source = Path(dep['manifest_path']).parent
            destination = notices / f"{dep['name']}-{dep['version']}"
            destination.mkdir()
            files = []
            for entry in source.rglob('*'):
                if entry.is_file() and entry.name.upper().startswith(('LICENSE', 'COPYING', 'NOTICE', 'COPYRIGHT', 'AUTHORS')):
                    files.append(entry)
            if dep.get('license_file'):
                files.append(source / dep['license_file'])
            for file in set(files):
                relative = file.relative_to(source)
                target = destination / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(file, target)
            supplement = ROOT / 'licenses' / f"{dep['name']}-{dep['version']}"
            if supplement.is_dir():
                shutil.copytree(supplement, destination, dirs_exist_ok=True)
            if not any(destination.rglob('*')):
                raise SystemExit(f"No license notice found for {dep['name']} {dep['version']}")
            if dep.get('license') == 'MPL-2.0':
                # Ship the unmodified covered source alongside its license.
                shutil.copytree(source, destination / 'source',
                                ignore=shutil.ignore_patterns('.cargo-ok'))
            inventory.append({key: dep.get(key) for key in ['name', 'version', 'license', 'repository', 'source']})
        (notices / 'dependencies.json').write_text(json.dumps(inventory, indent=2) + '\n')
        (stage / 'BUILD-INFO.txt').write_text(
            command('rustc', '-Vv') + '\n' + platform.platform() + '\n\n' +
            command('ldd', str(binary)))
        with tempfile.TemporaryDirectory(prefix='blink-smoke-') as smoke:
            subprocess.run([str(stage / 'blink'), '--config', str(Path(smoke) / 'config.toml'),
                            '--plain'], cwd=smoke, check=True, stdout=subprocess.DEVNULL)
            subprocess.run([str(stage / 'blink'), '--list-images'], cwd=smoke,
                           check=True, stdout=subprocess.DEVNULL)
        archive = dist / f'{name}.tar.gz'
        with tarfile.open(archive, 'w:gz') as tar:
            tar.add(stage, arcname=name)
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        (dist / f'{name}.sha256').write_text(f'{digest}  {archive.name}\n')
        print(archive)


if __name__ == '__main__':
    main()
