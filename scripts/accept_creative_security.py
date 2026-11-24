"""Explicit negative broker fixtures against a synthetic acceptance workspace."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from PIL import Image


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--agent', type=Path, required=True)
    parser.add_argument('--workspace', type=Path, required=True)
    args = parser.parse_args()
    root = args.workspace.resolve()
    original = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in root.iterdir() if p.is_file()}
    frames = [Image.new('RGB', (20, 20), c) for c in ('red', 'blue')]
    frames[0].save(root / 'unsafe-animated.png', save_all=True, append_images=frames[1:], duration=100)
    Image.new('1', (4001, 4001)).save(root / 'unsafe-oversized.png')
    (root / 'unsafe.svg').write_text('<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script><image href="https://example.invalid/private"/></svg>')
    rows = []
    cases = [
        ('parent traversal', dict(operation='image_inspect', input='../private.png')),
        ('absolute path', dict(operation='image_inspect', input=str(root / 'original-landscape.jpg'))),
        ('protected path', dict(operation='image_inspect', input='.env')),
        ('overwrite', dict(operation='image_prepare', input='original-landscape.jpg', output='original-landscape.jpg', width=100, height=100, mode='cover')),
        ('invalid output extension', dict(operation='image_prepare', input='original-landscape.jpg', output='unsafe.exe', width=100, height=100, mode='cover')),
        ('unexpected request fields', dict(operation='contrast', foreground='#000000', background='#FFFFFF', command='whoami')),
        ('arbitrary executable instructions', dict(operation='execute_script', input='unsafe.svg')),
        ('raw SVG input', dict(operation='svg_compose', input='unsafe.svg', output='unsafe-output.png')),
        ('animated image', dict(operation='image_inspect', input='unsafe-animated.png')),
        ('animated colour extraction', dict(operation='extract', input='unsafe-animated.png', count=2)),
        ('decoded image pixel cap', dict(operation='image_inspect', input='unsafe-oversized.png')),
        ('cloud vision model', dict(operation='vision_inspect', input='original-landscape.jpg', model='minimax-m3:cloud', mode='describe')),
        ('unsupported vision mode', dict(operation='vision_inspect', input='original-landscape.jpg', model='qwen3.5:4b', mode='execute')),
    ]
    for name, scene in [
        ('unapproved image', {'width': 200, 'height': 200, 'elements': [{'id': 'i', 'kind': 'image', 'x': 0, 'y': 0, 'w': 100, 'h': 100, 'asset': 0, 'fit': 'contain'}]}),
        ('external SVG URL', {'width': 200, 'height': 200, 'elements': [{'id': 'r', 'kind': 'rect', 'x': 0, 'y': 0, 'w': 100, 'h': 100, 'fill': 'url(https://example.invalid)'}]}),
        ('executable scene element', {'width': 200, 'height': 200, 'elements': [{'id': 's', 'kind': 'script', 'text': 'whoami'}]}),
    ]:
        source = 'unsafe-' + name.replace(' ', '-') + '.json'
        (root / source).write_text(json.dumps(scene))
        cases.append((name, dict(operation='svg_compose', input=source, output=source + '.png')))
    if (root / 'redirect').exists():
        cases.append(('filesystem redirection', dict(operation='image_inspect', input='redirect/private.png')))
    for name, request in cases:
        result = subprocess.run([str(args.agent.resolve()), '--design-operation', json.dumps(request), str(root)], capture_output=True, text=True, encoding='utf-8', timeout=135)
        row = {'case': name, 'rejected': result.returncode != 0, 'error': result.stderr.strip()}
        rows.append(row)
        print(name, 'PASS' if row['rejected'] else 'FAIL', flush=True)
    preserved = all((root / name).is_file() and hashlib.sha256((root / name).read_bytes()).hexdigest() == digest for name, digest in original.items())
    report = {'cases': rows, 'passed': all(r['rejected'] for r in rows) and preserved, 'original_files_preserved': preserved,
              'scope': 'Synthetic negative requests; separate Rust tests cover review drift, cancellation, grants and changed runtimes. Not an OS sandbox.'}
    (root / 'security-report.json').write_text(json.dumps(report, indent=2))
    return int(not report['passed'])


if __name__ == '__main__':
    raise SystemExit(main())
