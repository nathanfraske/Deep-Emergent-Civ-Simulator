# GPU counterpart to crates/core/examples/diffusion_bench.rs: the same canonical fixed-point Jacobi
# heat-diffusion stencil (Part 5.5 workload), run on the GPU. Prints the same checksum (must match the
# CPU bit-for-bit) and the throughput. Spike (cupy/NVRTC); the production kernel is a CubeCL #[cube]
# function (the confirmed backend). This cupy spike was the quickest on-device check; crates.io is
# reachable, so the production kernel is CubeCL.
#   ./gpuvenv/bin/python docs/working/gpu_diffusion.py
import numpy as np, cupy as cp
props = cp.cuda.runtime.getDeviceProperties(0)
print("device:", props["name"].decode() if isinstance(props["name"], bytes) else props["name"])

N = 1024
ITERS = 200
K = (1 << 32) // 5  # from_ratio(1,5), truncated, matching Fixed

KERNEL = r'''
extern "C" __global__ void diffuse(const long long* F, long long* G, int n, long long k){
    int x = blockIdx.x*blockDim.x + threadIdx.x;
    int y = blockIdx.y*blockDim.y + threadIdx.y;
    if (x>=n || y>=n) return;
    int yu = ((y+n-1)%n)*n, yd = ((y+1)%n)*n, yc = y*n;
    int xl = (x+n-1)%n, xr = (x+1)%n;
    long long c = F[yc+x];
    long long lap = F[yu+x] + F[yd+x] + F[yc+xl] + F[yc+xr] - (c*4);
    long long kl = (long long)(((__int128)k * (__int128)lap) >> 32); // floor, matches Fixed::mul
    G[yc+x] = c + kl;
}'''
diff = cp.RawKernel(KERNEL, "diffuse", options=("--device-int128",))

# identical initial field to the CPU
xs = (np.arange(N) * 7)[None, :]
ys = (np.arange(N) * 13)[:, None]
f0 = ((xs + ys) % 100).astype(np.int64) << 32
F = cp.asarray(np.broadcast_to(f0, (N, N)).copy().reshape(-1))
G = cp.empty_like(F)

tpb = (16, 16)
bpg = ((N + 15)//16, (N + 15)//16)
# warm
diff(bpg, tpb, (F, G, np.int32(N), np.int64(K))); F, G = G, F
cp.cuda.Device().synchronize()

# reset field and time
F = cp.asarray(np.broadcast_to(f0, (N, N)).copy().reshape(-1)); G = cp.empty_like(F)
st = cp.cuda.Event(); en = cp.cuda.Event(); st.record()
for _ in range(ITERS):
    diff(bpg, tpb, (F, G, np.int32(N), np.int64(K)))
    F, G = G, F
en.record(); en.synchronize()
ms = cp.cuda.get_elapsed_time(st, en)
cells = N*N*ITERS
host = cp.asnumpy(F)
checksum = 0
for v in host:
    checksum ^= int(v)
checksum &= (1 << 64) - 1
if checksum >= (1 << 63):
    checksum -= (1 << 64)
print(f"GPU diffusion {N}x{N} x{ITERS}: {cells/(ms/1000)/1e6:,.0f} Mcell-updates/s ({ms/1000:.3f}s), checksum {checksum:#x}")
