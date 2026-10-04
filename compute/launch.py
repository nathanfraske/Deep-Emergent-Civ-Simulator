"""Extended v2 native cloud evolution using the measured allocated host budget."""
from pathlib import Path,PurePosixPath
import hashlib,json,os,shutil,signal,subprocess,sys,tarfile,tempfile,time,traceback
BASE=Path(__file__).resolve().parent;OUT=BASE/'results'
STOP=False;RSS_CAP=0;CPU_WORKERS=1

def sha(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for b in iter(lambda:f.read(1024**2),b''):h.update(b)
    return h.hexdigest()

def budget():
    workers=len(os.sched_getaffinity(0));cpu=Path('/sys/fs/cgroup/cpu.max')
    if cpu.exists():
        quota,period=cpu.read_text().split()
        if quota!='max':workers=min(workers,max(1,int(quota)//int(period)))
    if os.environ.get('FLEET_CPU_COUNT','').isdecimal():workers=min(workers,int(os.environ['FLEET_CPU_COUNT']))
    assert 1<=workers<=256
    mem={k:int(v.split()[0])*1024 for k,v in (line.split(':',1) for line in Path('/proc/meminfo').read_text().splitlines())}
    limit=min(mem['MemAvailable'],mem['MemTotal']);cg=Path('/sys/fs/cgroup/memory.max')
    if cg.exists() and cg.read_text().strip().isdecimal():limit=min(limit,int(cg.read_text().strip()))
    limit-=4*1024**3;assert limit>=4*1024**3,'Allocated memory too small'
    return workers,limit,mem

def rss(group):
    total=0
    for path in Path('/proc').iterdir():
        if not path.name.isdecimal():continue
        try:
            if os.getpgid(int(path.name))!=group:continue
            total+=int(next(v.split()[1] for v in (path/'status').read_text().splitlines() if v.startswith('VmRSS:')))*1024
        except (OSError,ProcessLookupError,PermissionError,StopIteration):pass
    return total

def stop(signum,frame):
    global STOP
    STOP=True

def run(command,folder,hard_wall,receipt='stdout.txt'):
    folder.mkdir(exist_ok=False);start=time.monotonic();peak=0;reason=None;last=start
    with (folder/receipt).open('wb') as out,(folder/'stderr.txt').open('wb') as err,(folder/'monitor.jsonl').open('w') as monitor:
        p=subprocess.Popen(command,stdout=out,stderr=err,start_new_session=True)
        while p.poll() is None:
            now=time.monotonic();used=rss(p.pid);peak=max(peak,used)
            size=sum(v.stat().st_size for v in OUT.rglob('*') if v.is_file());free=shutil.disk_usage(BASE).free
            if used>RSS_CAP:reason='summed_rss_guard'
            elif size>8*1024**3:reason='evidence_byte_guard'
            elif free<8*1024**3:reason='disk_floor'
            elif now-start>hard_wall:reason='hard_wall'
            elif STOP:reason='interrupted'
            if reason:
                os.killpg(p.pid,signal.SIGTERM)
                try:p.wait(timeout=15)
                except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait()
                break
            if now-last>=30:
                last=now
                monitor.write(json.dumps(dict(elapsed_s=now-start,summed_rss_bytes=used,evidence_bytes=size,free_bytes=free))+'\n');monitor.flush()
                with (folder/'stderr.txt').open('rb') as f:
                    f.seek(max(0,(folder/'stderr.txt').stat().st_size-4000));lines=f.read().decode(errors='replace').splitlines()
                print('CIVSIM_NATIVE_PROGRESS '+folder.name+' '+(lines[-1] if lines else 'starting'),flush=True)
            time.sleep(1)
    record=dict(command=command,exit_code=124 if reason else p.returncode,process_exit_code=p.returncode,
                elapsed_s=time.monotonic()-start,resource_stop=reason,peak_summed_rss_bytes=peak,
                caps=dict(summed_rss_bytes=RSS_CAP,cpu_workers=CPU_WORKERS,evidence_bytes=8*1024**3,disk_floor_bytes=8*1024**3,hard_wall_s=hard_wall),
                files={v.name:dict(bytes=v.stat().st_size,sha256=sha(v)) for v in folder.iterdir() if v.is_file()})
    (folder/'status.json').write_text(json.dumps(record,indent=2)+'\n');return record

def main():
    global CPU_WORKERS,RSS_CAP
    assert not OUT.exists();OUT.mkdir()
    signal.signal(signal.SIGTERM,stop);signal.signal(signal.SIGINT,stop)
    code=2;scratch=None
    try:
        CPU_WORKERS,RSS_CAP,memory=budget();os.environ['CIVSIM_FIELD_WORKERS']=str(CPU_WORKERS)
        assert shutil.disk_usage(BASE).free>=12*1024**3
        host=dict(host=os.uname().nodename,workers=CPU_WORKERS,summed_rss_cap_bytes=RSS_CAP,meminfo=memory,python=sys.version,
                  job={k:os.environ.get(k) for k in ('GITHUB_RUN_ID','GITHUB_RUN_ATTEMPT','GITHUB_JOB','GITHUB_SHA','RUNNER_NAME','FLEET_CPU_COUNT','FLEET_MEMORY_MIB')},
                  scope='V2 native CPU/RAM allocation, with 4 GiB guest memory headroom. No GPU exposure or pooled-VRAM claim.')
        (OUT/'host.json').write_text(json.dumps(host,indent=2)+'\n');print('CIVSIM_NATIVE_HOST '+json.dumps(host),flush=True)
        manifest=json.loads((BASE/'payload.json').read_text());assert sha(BASE/'payload.tar.gz')==manifest['payload_sha256']
        (OUT/'payload.json').write_text(json.dumps(manifest,indent=2)+'\n')
        with tempfile.TemporaryDirectory(prefix='civsim-native-heavy-') as directory:
            scratch=Path(directory)
            with tarfile.open(BASE/'payload.tar.gz','r:gz') as archive:
                members=archive.getmembers();assert len(members)==len(manifest['files']) and {m.name for m in members}==set(manifest['files'])
                assert sum(m.size for m in members)<64*1024**2
                for member in members:
                    p=PurePosixPath(member.name);assert member.isfile() and not p.is_absolute() and '..' not in p.parts and '\\' not in member.name
                    raw=archive.extractfile(member).read();wanted=manifest['files'][member.name]
                    assert len(raw)==wanted['bytes'] and hashlib.sha256(raw).hexdigest()==wanted['sha256']
                    path=scratch/member.name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
            source=scratch/'source';helper=scratch/'backend';binary=None
            compiler=shutil.which('g++') or shutil.which('c++')
            if compiler:
                binary=OUT/'field_worker'
                info=subprocess.run([compiler,'--version'],capture_output=True,check=True,timeout=30)
                (OUT/'compiler.txt').write_bytes(info.stdout+info.stderr)
                build=run([compiler,'-O3','-std=c++17','-pthread','-DCPU_ONLY',str(helper/'field_worker.cpp'),'-o',str(binary)],OUT/'build',600)
                if build['exit_code']:raise RuntimeError('Exact CPU worker compilation failed')
                controls=run([sys.executable,'-B',str(helper/'check_worker.py'),'--source-root',str(source),'--binary',str(binary),'--output',str(OUT/'native-controls')],OUT/'controls-command',900)
                if controls['exit_code']:raise RuntimeError('Exact CPU/full-gas controls failed; long evolution not started')
            else:
                (OUT/'optional-proposer.json').write_text(json.dumps(dict(compiler_available=False,mode='unchanged_canonical_packed_cpu',scope='Performance option unavailable; original independent model remains active.'))+'\n')
            inputs=OUT/'inputs';inputs.mkdir()
            for src,dst in ((source/'declared-controls/compact.state','state'),(source/'declared-controls/long.numerics','numerics'),(source/'floor.json','floor.json')):shutil.copyfile(src,inputs/dst)
            case=OUT/'compact-native-heavy128'
            command=[sys.executable,'-B',str(helper/'run_native.py'),'--source-root',str(source),'--device-record-dir',str(OUT/'backend-record')]
            if binary:command+=['--device-binary',str(binary)]
            command+=['--model','axisymmetric_bate1998_barotropic_finite_cloud_v1','--state',str(inputs/'state'),'--numerics',str(inputs/'numerics'),'--floor-receipt',str(inputs/'floor.json'),
                      '--journal',str(case/'frames.jsonl'),'--kernel-cache',str(case/'kernel.json'),'--workers',str(CPU_WORKERS),'--wall-seconds','43200']
            result=run(command,case,50400,'receipt.json');code=result['exit_code']
            for path in inputs.iterdir():shutil.copyfile(path,case/path.name)
            (case/'execution-backend.json').write_text(json.dumps(dict(mode='exact_native_cpu_candidate' if binary else 'canonical_packed_cpu',workers=CPU_WORKERS,
                binary_sha256=sha(binary) if binary else None,wrapper_sha256=sha(helper/'run_native.py'),
                scope='Same declared original gas and independent stage checks; numerical work/depth limits are separately declared.'),indent=2)+'\n')
    except Exception:
        (OUT/'driver-error.txt').write_text(traceback.format_exc());code=2
    finally:
        assert scratch is None or not scratch.exists()
        (OUT/'cleanup.json').write_text(json.dumps(dict(temporary_source_removed=scratch is None or not scratch.exists(),temporary_source=str(scratch),free_bytes=shutil.disk_usage(BASE).free))+'\n')
        files={str(v.relative_to(OUT)):dict(bytes=v.stat().st_size,sha256=sha(v)) for v in OUT.rglob('*') if v.is_file()}
        (OUT/'completion.json').write_text(json.dumps(dict(schema='civsim.native-heavy-job.v1',exit_code=code,files=files,formed_disk=False,thermal_star=False,
                 scope='Bounded conditional gas evolution; host execution is separate from physical admission.'),indent=2)+'\n')
        print('CIVSIM_NATIVE_COMPLETE '+str(code),flush=True)
    return 0 if code==0 else 1
if __name__=='__main__':raise SystemExit(main())
