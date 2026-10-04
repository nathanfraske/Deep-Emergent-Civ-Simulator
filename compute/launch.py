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
            lines=(p/'smaps_rollup').read_text().splitlines()
            total+=int(next(s.split()[1] for s in lines if s.startswith('Pss:')))*1024
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
            if used>48*1024**3:reason='physical_pss_guard'
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
                monitor.write(json.dumps(dict(elapsed_s=now-began,group_pss_bytes=used,evidence_bytes=size,free_bytes=shutil.disk_usage(BASE).free))+'\n');monitor.flush()
                print('CIVSIM_B70_PROGRESS '+folder.name+' '+(lines[-1] if lines else 'starting'),flush=True)
            time.sleep(1)
    record=dict(command=command,exit_code=124 if reason else process.returncode,process_exit_code=process.returncode,
                elapsed_s=time.monotonic()-began,resource_stop=reason,peak_group_pss_bytes=peak,
                caps=dict(group_pss_bytes=48*1024**3,evidence_bytes=8*1024**3,disk_floor_bytes=8*1024**3,hard_wall_s=hard_wall),
                files={p.name:dict(bytes=p.stat().st_size,sha256=sha(p)) for p in folder.iterdir() if p.is_file()})
    (folder/'status.json').write_text(json.dumps(record,indent=2)+'\n')
    return record


def main():
    assert not OUT.exists();OUT.mkdir();signal.signal(signal.SIGTERM,stop);signal.signal(signal.SIGINT,stop)
    code=2;scratch=None;study_process=None
    try:
        assert shutil.disk_usage(BASE).free>=12*1024**3
        packet=BASE/'payload.tar.gz';manifest=json.loads((BASE/'payload.json').read_text());assert sha(packet)==manifest['payload_sha256']
        (OUT/'payload.json').write_text(json.dumps(manifest,indent=2)+'\n')
        (OUT/'host.json').write_text(json.dumps(dict(cpu_affinity=len(os.sched_getaffinity(0)),meminfo=Path('/proc/meminfo').read_text(),runner=os.environ.get('RUNNER_NAME'),allocation='Dedicated B70 pair: 16 CPUs, 64 GiB RAM. Gas uses 8 arithmetic workers beside 7 separate cold-origin studies; memory guard is physical PSS.'),indent=2)+'\n')
        with tempfile.TemporaryDirectory(prefix='civsim-parallel-b70-') as directory:
            scratch=Path(directory)
            with tarfile.open(packet,'r:gz') as archive:
                members=archive.getmembers();assert len(members)==len(manifest['files']) and {m.name for m in members}==set(manifest['files'])
                for m in members:
                    path=PurePosixPath(m.name);assert m.isfile() and not path.is_absolute() and '..' not in path.parts and '\\' not in m.name
                    raw=archive.extractfile(m).read();held=manifest['files'][m.name];assert len(raw)==held['bytes'] and hashlib.sha256(raw).hexdigest()==held['sha256']
                    target=scratch/m.name;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw)
            source=scratch/'source';os.environ['PYTHONPATH']=str(source);os.environ['OPENBLAS_NUM_THREADS']='1'
            if not Path('/usr/include/gmpxx.h').exists() or not shutil.which('g++'):
                if os.geteuid()!=0:raise RuntimeError('GMP development package unavailable in job environment')
                if run(['apt-get','update','-qq'],OUT/'dependency-index',600)['exit_code']:raise RuntimeError('package index failure')
                if run(['apt-get','install','-y','-qq','g++','libgmp-dev'],OUT/'dependency-install',600)['exit_code']:raise RuntimeError('GMP build dependency failure')
            binary=OUT/'exact_reduce';gpu=OUT/'field_worker'
            if run(['g++','-O3','-std=c++17','-fopenmp',str(source/'scripts/compute/exact_reduce.cpp'),'-lgmpxx','-lgmp','-o',str(binary)],OUT/'cpu-build',600)['exit_code']:raise RuntimeError('parallel CPU build failed')
            if run(['icpx','-O3','-std=c++17','-pthread','-fsycl',str(source/'scripts/compute/field_worker.cpp'),'-o',str(gpu)],OUT/'gpu-build',600)['exit_code']:raise RuntimeError('profiled B70 build failed')
            if run([sys.executable,'-B','-m','scripts.compute.test_exact_parallel',str(binary),str(OUT/'parallel-controls'),'--workers','8'],OUT/'parallel-controls-command',900)['exit_code']:raise RuntimeError('parallel exact/full-gas controls failed')
            if run([sys.executable,'-B',str(source/'scripts/compute/check_device.py'),'--source-root',str(source),'--binary',str(gpu),'--output',str(OUT/'device-controls')],OUT/'device-controls-command',900)['exit_code']:raise RuntimeError('B70 exact/full-gas controls failed')
            # The study controller owns and closes its seven bounded child jobs.
            study_script=scratch/'study_driver.py'
            study_script.write_text("import sys\nfrom scripts.stellar_formation.sweep import main\nif __name__=='__main__': main()\n")
            study_out=OUT/'thermal-study';study_log=(OUT/'thermal-study.stdout.txt').open('wb');study_err=(OUT/'thermal-study.stderr.txt').open('wb')
            study_process=subprocess.Popen([sys.executable,'-B',str(study_script),str(source/'examples/stellar-formation/refinement-study.json'),str(study_out),'--workers','7'],stdout=study_log,stderr=study_err,start_new_session=True)
            case=OUT/'rotating-parallel-gas';inputs=OUT/'inputs';inputs.mkdir()
            for a,b in (('declared-controls/compact.state','state'),('declared-controls/long.numerics','numerics'),('floor.json','floor.json')):shutil.copyfile(source/a,inputs/b)
            command=[sys.executable,'-B','-m','scripts.compute.run_gas','--exact-binary',str(binary),'--gpu-binary',str(gpu),'--gpu-policy','adaptive','--execution-workers','8','--execution-record-dir',str(OUT/'execution'),'--model','axisymmetric_bate1998_barotropic_finite_cloud_v1','--state',str(inputs/'state'),'--numerics',str(inputs/'numerics'),'--floor-receipt',str(inputs/'floor.json'),'--journal',str(case/'frames.jsonl'),'--kernel-cache',str(case/'kernel.json'),'--workers','8','--wall-seconds','7200']
            result=run(command,case,9000,'receipt.json');code=result['exit_code']
            for p in inputs.iterdir():shutil.copyfile(p,case/p.name)
            # Seven studies have a 75-minute individual bound. Keep their final
            # receipts and independently checked saved histories if gas ends early.
            try:study_process.wait(timeout=4800)
            except subprocess.TimeoutExpired:os.killpg(study_process.pid,signal.SIGTERM);study_process.wait(timeout=30)
            study_log.close();study_err.close()
            (OUT/'thermal-study-exit.json').write_text(json.dumps(dict(exit_code=study_process.returncode))+'\n')
    except Exception:
        (OUT/'driver-error.txt').write_text(traceback.format_exc());code=2
    finally:
        if study_process is not None and study_process.poll() is None:
            os.killpg(study_process.pid,signal.SIGTERM)
            try:study_process.wait(timeout=30)
            except subprocess.TimeoutExpired:os.killpg(study_process.pid,signal.SIGKILL);study_process.wait()
        (OUT/'cleanup.json').write_text(json.dumps(dict(owned_temporary_source_removed=scratch is None or not scratch.exists(),temporary_source=str(scratch),free_bytes=shutil.disk_usage(BASE).free))+'\n')
        files={str(p.relative_to(OUT)):dict(bytes=p.stat().st_size,sha256=sha(p)) for p in OUT.rglob('*') if p.is_file()}
        (OUT/'completion.json').write_text(json.dumps(dict(schema='civsim.parallel-b70-job.v1',exit_code=code,files=files,stellar_admission=False,completed_system=False,scope='Exact parallel execution and separate cold gas studies. Actual device use is in profiling counters; no overall speed or physical star classification is inferred.'),indent=2)+'\n')
    return 0 if code==0 else 1
if __name__=='__main__':raise SystemExit(main())
