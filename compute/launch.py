"""B70 pair controls and useful long gas evolution, with owned cleanup."""
from pathlib import Path, PurePosixPath
import hashlib, json, os, shutil, signal, subprocess, sys, tarfile, tempfile, time, traceback

BASE=Path(__file__).resolve().parent
OUT=BASE/'results';STOP=False
def sha(p):
    h=hashlib.sha256()
    with p.open('rb') as f:
        for b in iter(lambda:f.read(1024**2),b''):h.update(b)
    return h.hexdigest()
def rss(group):
    total=0
    for p in Path('/proc').iterdir():
        if not p.name.isdecimal():continue
        try:
            if os.getpgid(int(p.name))!=group:continue
            lines=(p/'status').read_text().splitlines()
            total+=int(next(s.split()[1] for s in lines if s.startswith('VmRSS:')))*1024
        except (OSError,ProcessLookupError,PermissionError,StopIteration):pass
    return total
def stop(signum,frame):
    global STOP
    STOP=True
def run(command,folder,hard_wall,receipt='stdout.txt'):
    folder.mkdir(exist_ok=False);began=time.monotonic();peak=0;reason=None;last=began
    with (folder/receipt).open('wb') as stdout,(folder/'stderr.txt').open('wb') as stderr,(folder/'monitor.jsonl').open('w') as monitor:
        process=subprocess.Popen(command,stdout=stdout,stderr=stderr,start_new_session=True)
        while process.poll() is None:
            now=time.monotonic();used=rss(process.pid);peak=max(peak,used)
            size=sum(p.stat().st_size for p in OUT.rglob('*') if p.is_file())
            if used>48*1024**3:reason='summed_rss_guard'
            elif size>8*1024**3:reason='evidence_byte_guard'
            elif shutil.disk_usage(BASE).free<8*1024**3:reason='disk_floor'
            elif now-began>hard_wall:reason='hard_wall'
            elif STOP:reason='interrupted'
            if reason:
                os.killpg(process.pid,signal.SIGTERM)
                try:process.wait(timeout=15)
                except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGKILL);process.wait()
                break
            if now-last>=30:
                last=now;lines=(folder/'stderr.txt').read_text(errors='replace').splitlines()
                monitor.write(json.dumps(dict(elapsed_s=now-began,summed_rss_bytes=used,evidence_bytes=size,free_bytes=shutil.disk_usage(BASE).free))+'\n');monitor.flush()
                print('CIVSIM_B70_PROGRESS '+folder.name+' '+(lines[-1] if lines else 'starting'),flush=True)
            time.sleep(1)
    record=dict(command=command,exit_code=124 if reason else process.returncode,process_exit_code=process.returncode,
                elapsed_s=time.monotonic()-began,resource_stop=reason,peak_summed_rss_bytes=peak,
                caps=dict(summed_rss_bytes=48*1024**3,evidence_bytes=8*1024**3,disk_floor_bytes=8*1024**3,hard_wall_s=hard_wall),
                files={p.name:dict(bytes=p.stat().st_size,sha256=sha(p)) for p in folder.iterdir() if p.is_file()})
    (folder/'status.json').write_text(json.dumps(record,indent=2)+'\n')
    return record

def main():
    assert not OUT.exists();OUT.mkdir()
    signal.signal(signal.SIGTERM,stop);signal.signal(signal.SIGINT,stop)
    packet=BASE/'payload.tar.gz';manifest=json.loads((BASE/'payload.json').read_text())
    assert sha(packet)==manifest['payload_sha256']
    assert shutil.disk_usage(BASE).free>=12*1024**3
    (OUT/'host.json').write_text(json.dumps(dict(host=os.uname().nodename,cpu_affinity=len(os.sched_getaffinity(0)),meminfo=Path('/proc/meminfo').read_text(),python=sys.version,
        job={k:os.environ.get(k) for k in ('GITHUB_RUN_ID','GITHUB_SHA','RUNNER_NAME','FLEET_GPU_PROFILE')},
        memory_scope='Dedicated pair profile reserves 64 GiB host RAM. Actual summed allocation is guarded at 48 GiB; device virtual address mappings are not capped as physical RAM.'),indent=2)+'\n')
    (OUT/'payload.json').write_text(json.dumps(manifest,indent=2)+'\n')
    compiler=subprocess.run(['icpx','--version'],capture_output=True,check=True,timeout=30)
    (OUT/'compiler.txt').write_bytes(compiler.stdout+compiler.stderr)
    code=2;scratch=None
    try:
        with tempfile.TemporaryDirectory(prefix='civsim-b70-resident-') as directory:
            scratch=Path(directory)
            with tarfile.open(packet,'r:gz') as archive:
                members=archive.getmembers();assert len(members)==len(manifest['files'])
                assert {m.name for m in members}==set(manifest['files'])
                assert sum(m.size for m in members)<64*1024**2
                for member in members:
                    p=PurePosixPath(member.name);assert member.isfile() and not p.is_absolute() and '..' not in p.parts and '\\' not in member.name
                    raw=archive.extractfile(member).read();f=manifest['files'][member.name]
                    assert len(raw)==f['bytes'] and hashlib.sha256(raw).hexdigest()==f['sha256']
                    target=scratch/member.name;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw)
            helper=scratch/'backend';source=scratch/'source';binary=OUT/'field_worker'
            built=run(['icpx','-O3','-std=c++17','-pthread','-fsycl',str(helper/'field_worker.cpp'),'-o',str(binary)],OUT/'build',600)
            if built['exit_code']:raise RuntimeError('B70 compilation failed')
            controls=run([sys.executable,'-B',str(helper/'check_worker.py'),'--source-root',str(source),'--binary',str(binary),'--output',str(OUT/'device-controls')],OUT/'controls-command',900)
            if controls['exit_code']:raise RuntimeError('B70 exact/full-gas controls failed; long evolution not started')
            case=OUT/'compact-b70-long128'
            # The guard creates the case; inputs are kept outside until that creation.
            inputs=OUT/'inputs';inputs.mkdir()
            for src,dst in [(source/'declared-controls/compact.state','state'),(source/'declared-controls/long.numerics','numerics'),(source/'floor.json','floor.json')]:shutil.copyfile(src,inputs/dst)
            command=[sys.executable,'-B',str(helper/'run_device.py'),'--source-root',str(source),'--device-binary',str(binary),'--device-record-dir',str(OUT/'backend-record'),'--device-policy','adaptive',
                     '--model','axisymmetric_bate1998_barotropic_finite_cloud_v1','--state',str(inputs/'state'),'--numerics',str(inputs/'numerics'),'--floor-receipt',str(inputs/'floor.json'),
                     '--journal',str(case/'frames.jsonl'),'--kernel-cache',str(case/'kernel.json'),'--workers','16','--wall-seconds','14400']
            result=run(command,case,18000,'receipt.json');code=result['exit_code']
            for path in inputs.iterdir():shutil.copyfile(path,case/path.name)
            (case/'execution-backend.json').write_text(json.dumps(dict(binary_sha256=sha(binary),wrapper_sha256=sha(helper/'run_device.py'),adapter_sha256=sha(helper/'device_field.py'),worker_sha256=sha(helper/'field_worker.cpp'),
                scope='Additional receipt implementation bindings identify the experimental proposer; original independent stage and boundary checks remain active. Formed-disk and continuum claims remain open.'),indent=2)+'\n')
    except Exception:
        (OUT/'driver-error.txt').write_text(traceback.format_exc());code=2
    finally:
        assert scratch is None or not scratch.exists()
        (OUT/'cleanup.json').write_text(json.dumps(dict(temporary_source_removed=scratch is None or not scratch.exists(),temporary_source=str(scratch),free_bytes=shutil.disk_usage(BASE).free,scope='Only this invocation owned source/compiler scratch. Scientific case, controls, binary, failures and hashes are retained.'),indent=2)+'\n')
        files={str(p.relative_to(OUT)):dict(bytes=p.stat().st_size,sha256=sha(p)) for p in OUT.rglob('*') if p.is_file()}
        (OUT/'completion.json').write_text(json.dumps(dict(schema='civsim.b70-resident-job.v1',exit_code=code,files=files,formed_disk=False,completed_system=False,
            scope='Bounded evolving gas and exact arithmetic evidence. Successful host execution does not classify a disk or star.'),indent=2)+'\n')
        print('CIVSIM_B70_COMPLETE '+str(code),flush=True)
    return 0 if code==0 else 1

if __name__=='__main__':raise SystemExit(main())
