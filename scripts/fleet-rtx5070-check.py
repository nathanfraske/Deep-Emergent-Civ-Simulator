"""Small real CUDA computation; no toolkit or third-party Python packages required.

The driver JITs PTX and executes it on the explicitly selected RTX 5070.
Every result and the overrun guard are checked against an independent CPU loop.
This qualifies device execution, not a simulation backend or performance claim.
"""
import ctypes as c
import datetime
import hashlib
import json
import os
from pathlib import Path
import socket
import time

PTX = b"""
.version 6.0
.target sm_70
.address_size 64
.visible .entry verify_vector(
    .param .u64 output_ptr,
    .param .u32 length,
    .param .u32 seed)
{
    .reg .pred %p;
    .reg .b32 %r<10>;
    .reg .b64 %rd<4>;
    ld.param.u64 %rd1, [output_ptr];
    ld.param.u32 %r1, [length];
    ld.param.u32 %r2, [seed];
    mov.u32 %r3, %ctaid.x;
    mov.u32 %r4, %ntid.x;
    mov.u32 %r5, %tid.x;
    mad.lo.u32 %r6, %r3, %r4, %r5;
    setp.ge.u32 %p, %r6, %r1;
    @%p bra DONE;
    xor.b32 %r2, %r2, %r6;
    mov.u32 %r7, 0;
LOOP:
    shr.u32 %r8, %r2, 13;
    mad.lo.u32 %r2, %r2, 1664525, 1013904223;
    xor.b32 %r2, %r2, %r8;
    add.u32 %r7, %r7, 1;
    setp.lt.u32 %p, %r7, 64;
    @%p bra LOOP;
    mul.wide.u32 %rd2, %r6, 4;
    add.u64 %rd3, %rd1, %rd2;
    st.global.u32 [%rd3], %r2;
DONE:
    ret;
}
"""


def cpu_reference(index, seed):
    value = index ^ seed
    for _ in range(64):
        value = ((value * 1664525 + 1013904223) & 0xffffffff) ^ (value >> 13)
    return value


def main():
    library = Path('/usr/lib/wsl/lib/libcuda.so.1')
    if not library.exists() or not Path('/dev/dxg').exists():
        raise RuntimeError('WSL CUDA device is unavailable')
    driver = c.CDLL(str(library))
    pointer = c.c_void_p
    u32 = c.c_uint32
    u64 = c.c_uint64
    signatures = {
        'cuInit': [u32],
        'cuDeviceGetCount': [c.POINTER(c.c_int)],
        'cuDeviceGet': [c.POINTER(c.c_int), c.c_int],
        'cuDeviceGetName': [pointer, c.c_int, c.c_int],
        'cuDeviceGetUuid': [pointer, c.c_int],
        'cuDeviceComputeCapability': [c.POINTER(c.c_int), c.POINTER(c.c_int), c.c_int],
        'cuDriverGetVersion': [c.POINTER(c.c_int)],
        'cuDeviceTotalMem_v2': [c.POINTER(c.c_size_t), c.c_int],
        'cuDevicePrimaryCtxRetain': [c.POINTER(pointer), c.c_int],
        'cuDevicePrimaryCtxRelease_v2': [c.c_int],
        'cuCtxSetCurrent': [pointer],
        'cuCtxSynchronize': [],
        'cuModuleLoadData': [c.POINTER(pointer), pointer],
        'cuModuleGetFunction': [c.POINTER(pointer), pointer, c.c_char_p],
        'cuModuleUnload': [pointer],
        'cuMemAlloc_v2': [c.POINTER(u64), c.c_size_t],
        'cuMemsetD32_v2': [u64, u32, c.c_size_t],
        'cuMemcpyDtoH_v2': [pointer, u64, c.c_size_t],
        'cuMemFree_v2': [u64],
        'cuLaunchKernel': [pointer, u32, u32, u32, u32, u32, u32, u32,
                           pointer, c.POINTER(pointer), c.POINTER(pointer)],
        'cuGetErrorName': [c.c_int, c.POINTER(c.c_char_p)],
    }
    for name, arguments in signatures.items():
        method = getattr(driver, name)
        method.argtypes = arguments
        method.restype = c.c_int

    def call(name, *args):
        result = getattr(driver, name)(*args)
        if result:
            label = c.c_char_p()
            driver.cuGetErrorName(result, c.byref(label))
            raise RuntimeError(name + ': ' + (label.value.decode() if label.value else str(result)))

    call('cuInit', 0)
    count = c.c_int()
    call('cuDeviceGetCount', c.byref(count))
    if count.value != 1:
        raise RuntimeError('Expected one inventoried GPU')
    device = c.c_int()
    call('cuDeviceGet', c.byref(device), 0)
    name = c.create_string_buffer(256)
    call('cuDeviceGetName', name, len(name), device)
    if name.value.decode() != 'NVIDIA GeForce RTX 5070':
        raise RuntimeError('GPU identity differs')
    uuid = (c.c_ubyte * 16)()
    call('cuDeviceGetUuid', uuid, device)
    major, minor, version = c.c_int(), c.c_int(), c.c_int()
    call('cuDeviceComputeCapability', c.byref(major), c.byref(minor), device)
    call('cuDriverGetVersion', c.byref(version))
    memory = c.c_size_t()
    call('cuDeviceTotalMem_v2', c.byref(memory), device)
    context, module, function, allocation = pointer(), pointer(), pointer(), u64()
    call('cuDevicePrimaryCtxRetain', c.byref(context), device)
    try:
        call('cuCtxSetCurrent', context)
        source = c.create_string_buffer(PTX)
        call('cuModuleLoadData', c.byref(module), source)
        call('cuModuleGetFunction', c.byref(function), module, b'verify_vector')
        size, guard, seed = 262147, 8, 42
        result = (u32 * (size + guard))()
        call('cuMemAlloc_v2', c.byref(allocation), c.sizeof(result))
        call('cuMemsetD32_v2', allocation, 0xdeadbeef, size + guard)
        n, salt = u32(size), u32(seed)
        arguments = (pointer * 3)(c.addressof(allocation), c.addressof(n), c.addressof(salt))
        began = time.monotonic()
        call('cuLaunchKernel', function, (size + 255) // 256, 1, 1, 256, 1, 1, 0, None, arguments, None)
        call('cuCtxSynchronize')
        elapsed = time.monotonic() - began
        call('cuMemcpyDtoH_v2', result, allocation, c.sizeof(result))
        mismatches = sum(result[i] != cpu_reference(i, seed) for i in range(size))
        guard_ok = all(result[i] == 0xdeadbeef for i in range(size, size + guard))
        record = {
            'observed_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
            'hostname': socket.gethostname(), 'device': name.value.decode(),
            'device_uuid_hex': bytes(uuid).hex(), 'compute_capability': [major.value, minor.value],
            'cuda_driver_api_version': version.value, 'device_memory_bytes': memory.value,
            'elements': size, 'iterations_per_element': 64, 'mismatches': mismatches,
            'guard_unchanged': guard_ok, 'allocation_bytes': c.sizeof(result),
            'launch_and_synchronize_seconds': round(elapsed, 6),
            'result_sha256': hashlib.sha256(bytes(result)[:size * 4]).hexdigest(),
            'ptx_sha256': hashlib.sha256(PTX).hexdigest(),
            'source_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            'job': {key: os.environ.get(key) for key in
                    ('GITHUB_REPOSITORY', 'GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT',
                     'GITHUB_SHA', 'GITHUB_JOB', 'RUNNER_NAME')},
            'passed': mismatches == 0 and guard_ok,
        }
        print(json.dumps(record, indent=2), flush=True)
        if not record['passed']:
            raise RuntimeError('GPU numerical verification failed')
        return record
    finally:
        if allocation.value:
            call('cuMemFree_v2', allocation)
        if module.value:
            call('cuModuleUnload', module)
        call('cuDevicePrimaryCtxRelease_v2', device)


if __name__ == '__main__':
    main()
