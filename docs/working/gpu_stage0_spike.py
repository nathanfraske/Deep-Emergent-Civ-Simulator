# R-GPU-CANON-PIN Stage 0 device gate (spike; not a workspace crate, not on the canonical path).
#
# Confirms the pinned u32-limb Q32.32 multiply and divide (crates/core/tests/gpu_emulation.rs, the
# portable software gate) are bit-identical to the CPU oracle when JIT-compiled with NVRTC and run on
# a real NVIDIA GPU. RESULT (RTX 5090, driver 13.2, cupy-cuda12x runtime CUDA 12.9): PASS, 2,000,324
# multiply + 2,000,306 divide cases (all corners + 2M random each), zero mismatches.
#
# Stand-up (this cupy/NVRTC path was the quickest on-device check; crates.io is REACHABLE, so the
# production path is CubeCL, not cupy):
#   python3 -m venv gpuvenv && ./gpuvenv/bin/pip install "cupy-cuda12x[ctk]"
#   ./gpuvenv/bin/python docs/working/gpu_stage0_spike.py
#
# The same limb algorithm ports to a CubeCL #[cube] kernel unchanged; a multi-vendor run is the
# remaining Stage 0 confirmation, bit-identical by the integer-exactness argument.

#!/usr/bin/env python3
# R-GPU-CANON-PIN Stage 0 device gate: the pinned u32-limb Q32.32 multiply, JIT-compiled with NVRTC
# and run on the actual GPU via cupy, compared bit-for-bit against the exact oracle (Python big-int
# reproduction of Fixed::mul = ((a*b) >> 32) as i64). A pass proves the canonical limb kernel is
# bit-identical to the CPU oracle on real NVIDIA hardware; the same integer kernel is bit-identical
# across vendors by the exactness argument. No float anywhere on the canonical path.
import numpy as np
import cupy as cp

dev = cp.cuda.Device(0)
props = cp.cuda.runtime.getDeviceProperties(dev.id)
name = props["name"].decode() if isinstance(props["name"], bytes) else props["name"]
print(f"device: {name}")
print(f"cupy {cp.__version__}, runtime CUDA {cp.cuda.runtime.runtimeGetVersion()}, driver {cp.cuda.runtime.driverGetVersion()}")

# The pinned emulation, a faithful port of crates/core/tests/gpu_emulation.rs emu_mul, using only the
# shader-confined op set (u32 wrapping ops, u16*u16->u32, bitwise, shifts, comparisons; no 64-bit mul).
KERNEL = r'''
extern "C" __global__
void emu_mul(const long long* A, const long long* B, long long* OUT, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n) return;
    unsigned long long ua = (unsigned long long)A[idx];
    unsigned long long ub = (unsigned long long)B[idx];
    unsigned int alo = (unsigned int)ua, ahi = (unsigned int)(ua >> 32);
    unsigned int blo = (unsigned int)ub, bhi = (unsigned int)(ub >> 32);
    unsigned int a_neg = (ahi & 0x80000000u) ? 1u : 0u;
    unsigned int b_neg = (bhi & 0x80000000u) ? 1u : 0u;
    unsigned int neg = a_neg ^ b_neg;
    if (a_neg) { unsigned int nlo=(~alo)+1u; unsigned int c=(nlo==0u)?1u:0u; ahi=(~ahi)+c; alo=nlo; }
    if (b_neg) { unsigned int nlo=(~blo)+1u; unsigned int c=(nlo==0u)?1u:0u; bhi=(~bhi)+c; blo=nlo; }
    unsigned int aa[4]; aa[0]=alo&0xFFFFu; aa[1]=alo>>16; aa[2]=ahi&0xFFFFu; aa[3]=ahi>>16;
    unsigned int bb[4]; bb[0]=blo&0xFFFFu; bb[1]=blo>>16; bb[2]=bhi&0xFFFFu; bb[3]=bhi>>16;
    unsigned int acc[8];
    for (int i=0;i<8;i++) acc[i]=0u;
    for (int i=0;i<4;i++){
        unsigned int carry=0u;
        for (int j=0;j<4;j++){
            unsigned int t = aa[i]*bb[j] + acc[i+j] + carry;
            acc[i+j] = t & 0xFFFFu;
            carry = t >> 16;
        }
        int k=i+4;
        while (carry>0u){ unsigned int t=acc[k]+carry; acc[k]=t&0xFFFFu; carry=t>>16; k++; }
    }
    unsigned int w0=acc[0]|(acc[1]<<16);
    unsigned int w1=acc[2]|(acc[3]<<16);
    unsigned int w2=acc[4]|(acc[5]<<16);
    unsigned int w3=acc[6]|(acc[7]<<16);
    if (neg){
        unsigned int carry=1u;
        unsigned int v0=~w0, s0=v0+carry; carry=(s0<v0)?1u:0u; w0=s0;
        unsigned int v1=~w1, s1=v1+carry; carry=(s1<v1)?1u:0u; w1=s1;
        unsigned int v2=~w2, s2=v2+carry; carry=(s2<v2)?1u:0u; w2=s2;
        unsigned int v3=~w3, s3=v3+carry; w3=s3;
        (void)w0; (void)w3;
    }
    unsigned long long res = ((unsigned long long)w2 << 32) | (unsigned long long)w1;
    OUT[idx] = (long long)res;
}
'''
mod = cp.RawKernel(KERNEL, "emu_mul")

KERNEL_DIV = r'''
extern "C" __global__
void emu_div(const long long* A, const long long* B, long long* OUT, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n) return;
    unsigned long long ua=(unsigned long long)A[idx], ub=(unsigned long long)B[idx];
    unsigned int alo=(unsigned int)ua, ahi=(unsigned int)(ua>>32);
    unsigned int blo=(unsigned int)ub, bhi=(unsigned int)(ub>>32);
    unsigned int a_neg=(ahi&0x80000000u)?1u:0u, b_neg=(bhi&0x80000000u)?1u:0u, neg=a_neg^b_neg;
    if(a_neg){unsigned int nlo=(~alo)+1u; unsigned int c=(nlo==0u)?1u:0u; ahi=(~ahi)+c; alo=nlo;}
    if(b_neg){unsigned int nlo=(~blo)+1u; unsigned int c=(nlo==0u)?1u:0u; bhi=(~bhi)+c; blo=nlo;}
    unsigned int mdlo=blo, mdhi=bhi;
    unsigned int num0=0u, num1=alo, num2=ahi;
    unsigned int r0=0u,r1=0u,r2=0u,q0=0u,q1=0u;
    for(int i=95;i>=0;i--){
        unsigned int word=(i<32)?num0:((i<64)?num1:num2);
        unsigned int bit=(word>>(i&31))&1u;
        r2=(r2<<1)|(r1>>31); r1=(r1<<1)|(r0>>31); r0=(r0<<1)|bit;
        unsigned int ge=(r2!=0u)||(r1>mdhi)||((r1==mdhi)&&(r0>=mdlo));
        if(ge){
            unsigned int borrow0=(r0<mdlo)?1u:0u; r0=r0-mdlo;
            unsigned int b1a=(r1<mdhi)?1u:0u; unsigned int t1=r1-mdhi;
            unsigned int b1b=(t1<borrow0)?1u:0u; r1=t1-borrow0;
            r2=r2-(b1a|b1b);
            if(i<32) q0|=(1u<<i); else if(i<64) q1|=(1u<<(i-32));
        }
    }
    if(neg){ unsigned int nlo=(~q0)+1u; unsigned int c=(nlo==0u)?1u:0u; q1=(~q1)+c; q0=nlo; }
    unsigned long long res=((unsigned long long)q1<<32)|(unsigned long long)q0;
    OUT[idx]=(long long)res;
}
'''
mod_div = cp.RawKernel(KERNEL_DIV, "emu_div")

MASK = (1 << 64) - 1
def oracle(a, b):
    # exact Fixed::mul: arithmetic-shift floor then two's-complement narrow to i64
    r = (int(a) * int(b)) >> 32
    r &= MASK
    if r >= (1 << 63):
        r -= (1 << 64)
    return r

corners = [0,1,-1,2,-2,(1<<63)-1,-(1<<63),-(1<<63)+1,(1<<63)-2,
           1<<32,-(1<<32),(1<<32)+1,1<<31,-(1<<31),1<<62,-(1<<62),
           0x00000001FFFFFFFF,-0x0000000100000000]
ca=[]; cb=[]
for a in corners:
    for b in corners:
        ca.append(a); cb.append(b)

rng = np.random.default_rng(0xC0FFEE)
N = 2_000_000
ra = rng.integers(np.iinfo(np.int64).min, np.iinfo(np.int64).max, size=N, dtype=np.int64, endpoint=True)
rb = rng.integers(np.iinfo(np.int64).min, np.iinfo(np.int64).max, size=N, dtype=np.int64, endpoint=True)
A = np.concatenate([np.array(ca, dtype=np.int64), ra])
B = np.concatenate([np.array(cb, dtype=np.int64), rb])
n = A.size

dA = cp.asarray(A); dB = cp.asarray(B); dOut = cp.empty(n, dtype=cp.int64)
threads = 256; blocks = (n + threads - 1)//threads
mod((blocks,), (threads,), (dA, dB, dOut, np.int32(n)))
cp.cuda.Device().synchronize()
gpu = cp.asnumpy(dOut)

# Oracle: vectorized exact big-int via numpy object dtype, then narrow to i64.
prod = (A.astype(object) * B.astype(object)) >> 32
prod &= MASK
prod = np.where(prod >= (1 << 63), prod - (1 << 64), prod)
want = prod.astype(np.int64)

mism = int(np.count_nonzero(gpu != want))
print(f"cases: {n}  (corners {len(ca)} + random {N})")
print(f"mismatches (GPU limb-mul vs oracle): {mism}")
# spot-check the named corners against the closed-form oracle too
assert oracle(-(1<<63), -1) == (1<<31)
assert oracle(-(1<<63), -(1<<63)) == 0
print("named-corner oracle checks: PASS")
if mism:
    bad = np.nonzero(gpu != want)[0][:5]
    for i in bad:
        print(f"  MUL a={int(A[i]):#x} b={int(B[i]):#x} gpu={int(gpu[i]):#x} want={int(want[i]):#x}")

# --- Divide: same gate, over b != 0 ---
nz = B != 0
Ad = np.ascontiguousarray(A[nz]); Bd = np.ascontiguousarray(B[nz]); nd = Ad.size
dAd = cp.asarray(Ad); dBd = cp.asarray(Bd); dOutd = cp.empty(nd, dtype=cp.int64)
blocks_d = (nd + threads - 1)//threads
mod_div((blocks_d,), (threads,), (dAd, dBd, dOutd, np.int32(nd)))
cp.cuda.Device().synchronize()
gpud = cp.asnumpy(dOutd)
# oracle: trunc-toward-zero of (a<<32)/b, then narrow to i64.
NA = Ad.astype(object); NB = Bd.astype(object)
mag = (abs(NA) << 32) // abs(NB)
sgn = ((NA < 0) ^ (NB < 0))
q = np.where(sgn, -mag, mag)
q &= MASK
q = np.where(q >= (1 << 63), q - (1 << 64), q)
wantd = q.astype(np.int64)
mismd = int(np.count_nonzero(gpud != wantd))
print(f"divide cases: {nd} (b != 0)")
print(f"mismatches (GPU limb-div vs oracle): {mismd}")
if mismd:
    bad = np.nonzero(gpud != wantd)[0][:5]
    for i in bad:
        print(f"  DIV a={int(Ad[i]):#x} b={int(Bd[i]):#x} gpu={int(gpud[i]):#x} want={int(wantd[i]):#x}")

print("STAGE0_RESULT:", "PASS" if (mism == 0 and mismd == 0) else f"FAIL (mul {mism}, div {mismd})")
