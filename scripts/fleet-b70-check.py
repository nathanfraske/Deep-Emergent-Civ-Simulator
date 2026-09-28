#!/usr/bin/env python3
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.
"""Verify physical GPU selection, isolated execution and a compiled SYCL kernel."""
import datetime
import errno
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import time

PROFILES = {
    'b70-a': {'0000:04:00.0': 'renderD129'},
    'b70-b': {'0000:08:00.0': 'renderD130'},
    'b70-pair': {'0000:04:00.0': 'renderD129', '0000:08:00.0': 'renderD130'},
}


def verify_private_home(path=Path('/root')):
    """Accept absence or systemd's inaccessible mount, never an exposed home."""
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return {'kind': 'absent'}
    assert stat.S_ISDIR(metadata.st_mode) and metadata.st_uid == 0
    assert stat.S_IMODE(metadata.st_mode) == 0
    assert not os.access(path, os.R_OK | os.X_OK)
    mounts = [line.split() for line in Path('/proc/self/mountinfo').read_text().splitlines()]
    matches = [fields for fields in mounts if fields[4] == str(path)]
    assert len(matches) == 1
    fields = matches[0]
    assert fields[3].endswith('/systemd/inaccessible/dir')
    assert 'ro' in fields[5].split(',') and fields[fields.index('-') + 1] == 'tmpfs'
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    except OSError as error:
        assert error.errno == errno.EACCES
    else:
        os.close(descriptor)
        raise AssertionError('Private home can be opened')
    return {'kind': 'inaccessible-mount', 'mode': 0, 'uid': 0, 'open_denied': True}


def main():
    profile = os.environ['FLEET_GPU_PROFILE']
    selected = PROFILES[profile]
    assert os.geteuid() != 0
    assert int(os.environ['FLEET_GPU_COUNT']) == len(selected)
    assert set(os.environ['FLEET_GPU_PCI'].split(',')) == set(selected)
    assert {p.name for p in Path('/dev/dri').iterdir()} == set(selected.values())
    for pci, node in selected.items():
        assert (Path('/sys/class/drm') / node / 'device').resolve().name == pci
        fd = os.open('/dev/dri/' + node, os.O_RDWR)
        os.close(fd)
    private_home = verify_private_home()
    for hidden in ['/home/atcadmin', '/var/lib/test-fleet/protected',
                   '/var/lib/test-fleet/client-atc', '/dev/dri/renderD128']:
        assert not Path(hidden).exists(), hidden
    assert os.environ.get('ONEAPI_DEVICE_SELECTOR') == 'level_zero:gpu'
    source = Path(__file__).with_suffix('.cpp')
    output = Path('gpu-results')
    output.mkdir(exist_ok=True)
    executable = output / ('b70-' + profile)
    start = datetime.datetime.now(datetime.timezone.utc).isoformat()
    compiler = subprocess.check_output(['icpx', '--version'], text=True, timeout=20)
    subprocess.run(['icpx', '-fsycl', '-O2', '-std=c++17', str(source), '-o', str(executable)],
                   check=True, timeout=180)
    first = time.monotonic()
    samples = 0
    # A bounded overlap check lets the scheduler demonstrate independent cards.
    # This is a correctness window, not a throughput benchmark.
    while True:
        result = json.loads(subprocess.check_output([str(executable.resolve()),
            ','.join(sorted(selected))], text=True, timeout=40))
        assert result['passed'] and {d['pci'] for d in result['devices']} == set(selected)
        assert all(d['mismatches'] == 0 and d['guard_unchanged'] for d in result['devices'])
        samples += 1
        if profile == 'b70-pair' or time.monotonic() - first >= 45:
            break
        time.sleep(2)
    result.update(profile=profile, uid=os.geteuid(), selected_nodes=selected,
        private_home=private_home,
        started_utc=start, completed_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),
        samples=samples, compiler=compiler,
        source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
        wrapper_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        executable_sha256=hashlib.sha256(executable.read_bytes()).hexdigest(),
        job={key: os.environ[key] for key in ['GITHUB_REPOSITORY', 'GITHUB_RUN_ID',
             'GITHUB_RUN_ATTEMPT', 'GITHUB_SHA', 'GITHUB_JOB', 'RUNNER_NAME']})
    assert result['job']['GITHUB_REPOSITORY'] == 'nathanfraske/Deep-Emergent-Civ-Simulator'
    with (output / (profile + '.json')).open('x') as stream:
        json.dump(result, stream, indent=2)
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
