"""Exclusive ATC preparation, unordered useful work, and owned cleanup."""
from pathlib import Path,PurePosixPath
import argparse,hashlib,json,os,shutil,subprocess,sys,tarfile,time,traceback

BASE=Path(__file__).resolve().parent;OUT=BASE/'results';RUNTIME=BASE/'runtime';SOURCE=RUNTIME/'source'


def sha(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as f:
        for chunk in iter(lambda:f.read(1024**2),b''):h.update(chunk)
    return h.hexdigest()


def env():
    os.environ['PYTHONPATH']=str(SOURCE)
    if str(SOURCE) not in sys.path:sys.path.insert(0,str(SOURCE))
    for key in ('OPENBLAS_NUM_THREADS','MKL_NUM_THREADS','OMP_NUM_THREADS'):os.environ[key]='1'
    os.environ['OMP_SCHEDULE']='dynamic,1'


def command(command,folder,timeout):
    folder.mkdir(parents=True,exist_ok=False);began=time.monotonic()
    with (folder/'stdout.txt').open('wb') as stdout,(folder/'stderr.txt').open('wb') as stderr:
        try:result=subprocess.run(command,stdout=stdout,stderr=stderr,timeout=timeout)
        except subprocess.TimeoutExpired:
            (folder/'status.json').write_text(json.dumps(dict(exit_code=124,elapsed_s=time.monotonic()-began))+'\n');raise
    row=dict(command=command,exit_code=result.returncode,elapsed_s=time.monotonic()-began)
    (folder/'status.json').write_text(json.dumps(row,indent=2)+'\n')
    print('CIVSIM_PREPARATION '+json.dumps(row),flush=True)
    if result.returncode:raise RuntimeError('preparation failed: '+str(folder))


def extract():
    manifest=json.loads((BASE/'payload.json').read_text());assert sha(BASE/'payload.tar.gz')==manifest['payload_sha256']
    RUNTIME.mkdir(exist_ok=False)
    with tarfile.open(BASE/'payload.tar.gz','r:gz') as archive:
        members=archive.getmembers();assert len(members)==len(manifest['files']) and {m.name for m in members}==set(manifest['files'])
        for m in members:
            path=PurePosixPath(m.name);assert m.isfile() and not path.is_absolute() and '..' not in path.parts and '\\' not in m.name
            raw=archive.extractfile(m).read();fact=manifest['files'][m.name]
            assert len(raw)==fact['bytes'] and hashlib.sha256(raw).hexdigest()==fact['sha256']
            target=RUNTIME/m.name;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw)
    return manifest


def plans(probe=False):
    from scripts.compute.exact_reduce import available_cpus
    capacity=available_cpus();assert capacity>=16,'exclusive workstation did not expose enough CPU capacity'
    # Two gas cases and two six-process thermal solves fit a 22-CPU allocation.
    gas_slots=4;thermal_slots=max(4,(capacity-2*gas_slots)//2)
    result=[];folder=OUT/('probe' if probe else 'experiment');inputs=folder/'inputs';inputs.mkdir(parents=True)
    source_state=(SOURCE/'declared-controls/compact.state').read_text();source_num=(SOURCE/'declared-controls/long.numerics').read_text()
    gas_specs=[('rotating-original',16,16,5),('rotating-finer',16,32,4),('rotating-large',32,32,2)]
    for name,nr,nz,priority in (gas_specs[:1] if probe else gas_specs):
        state=inputs/(name+'.state');numerics=inputs/(name+'.numerics');state.write_text(source_state)
        text=source_num.replace('radial_cells=16','radial_cells='+str(nr)).replace('vertical_cells=16','vertical_cells='+str(nz))
        if probe:text=text.replace('max_steps=32768','max_steps=128').replace('sample_every=256','sample_every=16')
        numerics.write_text(text);case=folder/'cases'/name;case.mkdir(parents=True)
        if nr==nz==16:shutil.copyfile(SOURCE/'declared-controls/initial-kernel.json',case/'kernel.json')
        command=[sys.executable,'-B','-m','scripts.compute.run_gas','--exact-binary',str(OUT/'exact_reduce'),
                 '--gpu-binary',str(OUT/'field_worker'),'--gpu-policy','device','--execution-workers',str(gas_slots),
                 '--execution-record-dir',str(case/'execution'),'--model','axisymmetric_bate1998_barotropic_finite_cloud_v1',
                 '--state',str(state),'--numerics',str(numerics),'--floor-receipt',str(SOURCE/'floor.json'),
                 '--journal',str(case/'frames.jsonl'),'--kernel-cache',str(case/'kernel.json'),'--workers',str(gas_slots),
                 '--wall-seconds',str(45 if probe else 10800)]
        result.append(dict(name=name,kind='rotating',cpu_slots=gas_slots,priority=priority*4000,
                           estimate_seconds=4000,hard_wall_seconds=180 if probe else 14400,command=command))
    template=json.loads((SOURCE/'examples/stellar-formation/uniform-cloud.json').read_text())
    thermal_specs=[('thermal24',24,'.02',5),('thermal32',32,'.02',5),('thermal48',48,'.02',4),
                   ('thermal64',64,'.02',3),('thermal96',96,'.02',2),('thermal128',128,'.02',2),
                   ('thermal16-half',16,'.01',4),('thermal24-half',24,'.01',3),('thermal32-half',32,'.01',3),
                   ('thermal16-no-burning',16,'.02',4),('thermal24-no-burning',24,'.02',4),
                   ('thermal48-half',48,'.01',2),('thermal64-half',64,'.01',2)]
    for name,cells,change,priority in (thermal_specs[:3] if probe else thermal_specs):
        cfg=json.loads(json.dumps(template));cfg['numerics'].update(cells=cells,relative_step_change=change,
            max_wall_seconds=45 if probe else 10800,max_steps=100000,save_every=25)
        if 'no-burning' in name:cfg['interactions']['pp_i']=False
        cfg['state']['description']='Declared cold gas for an owner-authorized unordered ATC numerical experiment. No supplied star or requested endpoint.'
        path=inputs/(name+'.json');path.write_text(json.dumps(cfg,indent=2)+'\n')
        case=folder/'cases'/name;case.mkdir(parents=True)
        command=[sys.executable,'-B','-m','scripts.compute.thermal_parallel','--workers',str(thermal_slots),
                 '--unordered','--execution-record-dir',str(case/'execution'),str(path),str(case/'run')]
        result.append(dict(name=name,kind='thermal',cpu_slots=thermal_slots,priority=priority*1000,
                           estimate_seconds=1000,hard_wall_seconds=180 if probe else 10920,command=command))
    return result


def prepare():
    assert not OUT.exists();OUT.mkdir();probe=OUT/'probe';probe.mkdir()
    assert shutil.disk_usage(BASE).free>=16*1024**3
    manifest=extract();env()
    from scripts.compute.work_queue import physical_memory
    from scripts.compute.exact_reduce import available_cpus
    host=dict(cpu_affinity=len(os.sched_getaffinity(0)),allocated_cpus=available_cpus(),memory_bytes=physical_memory(),
              cgroup=Path('/proc/self/cgroup').read_text(),runner=os.environ.get('RUNNER_NAME'),
              fleet_cpu=os.environ.get('FLEET_CPU_COUNT'),fleet_memory_mib=os.environ.get('FLEET_MEMORY_MIB'),
              gpu_pci=os.environ.get('FLEET_GPU_PCI'),unordered=True,
              contract='fleet-gpu-b70-pair-all plus fleet-resources-all; exclusive workstation, measured capacity with host headroom')
    (probe/'host.json').write_text(json.dumps(host,indent=2)+'\n')
    (OUT/'payload.json').write_text(json.dumps(manifest,indent=2)+'\n')
    # Public LGPL headers are held in the packet. Link the installed replaceable
    # GMP runtime directly; the read-only sandbox needs no package installation.
    library=next((p for p in (Path('/usr/lib/x86_64-linux-gnu/libgmp.so.10'),Path('/usr/lib64/libgmp.so.10')) if p.exists()),None)
    if library is None:raise RuntimeError('installed GMP runtime missing')
    (probe/'gmp-runtime.json').write_text(json.dumps(dict(path=str(library),resolved=str(library.resolve()),sha256=sha(library)))+'\n')
    command(['g++','-O3','-std=c++17','-fopenmp','-I'+str(RUNTIME/'deps/include'),str(SOURCE/'scripts/compute/exact_reduce.cpp'),str(library),'-o',str(OUT/'exact_reduce')],probe/'cpu-build',300)
    command(['icpx','-O3','-std=c++17','-pthread','-fsycl',str(SOURCE/'scripts/compute/field_worker.cpp'),'-o',str(OUT/'field_worker')],probe/'gpu-build',600)
    command([sys.executable,'-B','-m','scripts.compute.test_exact_parallel',str(OUT/'exact_reduce'),str(probe/'exact-controls'),'--workers',str(min(16,available_cpus()))],probe/'exact-controls-command',300)
    command([sys.executable,'-B','-m','scripts.compute.check_device','--source-root',str(SOURCE),'--binary',str(OUT/'field_worker'),'--output',str(probe/'gpu-controls')],probe/'gpu-controls-command',300)
    command([sys.executable,'-B','-m','scripts.compute.test_thermal_parallel',str(probe/'thermal-controls'),'--workers','6'],probe/'thermal-controls-command',300)
    from scripts.compute.work_queue import execute
    result=execute(plans(True),probe/'queue',150)
    (probe/'preparation.json').write_text(json.dumps(dict(accepted=True,host=host,queue=result,
        exact_binary_sha256=sha(OUT/'exact_reduce'),gpu_binary_sha256=sha(OUT/'field_worker'),
        scope='Physical controls and short cold gas progress before the heavy unordered experiment. GPU timings are recorded per actual PCI address.'),indent=2)+'\n')


def experiment():
    env();assert json.loads((OUT/'probe/preparation.json').read_text())['accepted'] is True
    from scripts.compute.work_queue import execute
    result=execute(plans(False),OUT/'experiment/queue',21600)
    # Saved checks remain independent and are applied once to each available
    # thermal trajectory. A numerical refusal remains a recorded negative.
    for case in (OUT/'experiment/cases').glob('thermal*'):
        if (case/'run/receipt.json').exists():
            target=case/'independent';target.mkdir()
            with (target/'stdout.txt').open('wb') as stdout,(target/'stderr.txt').open('wb') as stderr:
                p=subprocess.run([sys.executable,'-B','-m','scripts.stellar_formation.watchdog',str(case/'run')],stdout=stdout,stderr=stderr,timeout=300)
            (target/'status.json').write_text(json.dumps(dict(exit_code=p.returncode))+'\n')
    (OUT/'experiment-result.json').write_text(json.dumps(result,indent=2)+'\n')


def cleanup():
    removed=False
    if RUNTIME.exists():
        resolved=RUNTIME.resolve();assert resolved.parent==BASE.resolve() and not RUNTIME.is_symlink()
        shutil.rmtree(resolved);removed=True
    OUT.mkdir(exist_ok=True)
    (OUT/'cleanup.json').write_text(json.dumps(dict(owned_runtime_removed=removed or not RUNTIME.exists(),free_bytes=shutil.disk_usage(BASE).free))+'\n')
    files={str(p.relative_to(OUT)):dict(bytes=p.stat().st_size,sha256=sha(p)) for p in OUT.rglob('*') if p.is_file()}
    (OUT/'completion.json').write_text(json.dumps(dict(schema='civsim.atc-all-evidence.v1',files=files,unordered=True,stellar_admission=False,completed_system=False),indent=2)+'\n')


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('phase',choices=('prepare','run','cleanup'));args=p.parse_args()
    try:{'prepare':prepare,'run':experiment,'cleanup':cleanup}[args.phase]()
    except Exception:
        OUT.mkdir(exist_ok=True);(OUT/(args.phase+'-error.txt')).write_text(traceback.format_exc());raise
