# GPU throughput for the canonical Q32.32 multiply, vs the CPU baseline (crates/core/examples/
# mul_throughput.rs). Times the native-i128 canonical kernel (what CUDA runs, bit-identical to the
# CPU Fixed::mul) and the portable u32-limb kernel (what a shader without i128 runs), over a large
# array on the GPU. Spike; not on the canonical path.
#   ./gpuvenv/bin/python docs/working/gpu_mul_throughput.py
import numpy as np, cupy as cp
props = cp.cuda.runtime.getDeviceProperties(0)
print("device:", props["name"].decode() if isinstance(props["name"], bytes) else props["name"])

NATIVE = r'''
extern "C" __global__ void mul_native(const long long* A, const long long* B, long long* O, int n){
    int i = blockIdx.x*blockDim.x+threadIdx.x; if(i>=n) return;
    O[i] = (long long)(((__int128)A[i] * (__int128)B[i]) >> 32);
}'''
LIMB = r'''
extern "C" __global__ void mul_limb(const long long* A, const long long* B, long long* O, int n){
    int idx=blockIdx.x*blockDim.x+threadIdx.x; if(idx>=n) return;
    unsigned long long ua=(unsigned long long)A[idx], ub=(unsigned long long)B[idx];
    unsigned int alo=(unsigned int)ua,ahi=(unsigned int)(ua>>32),blo=(unsigned int)ub,bhi=(unsigned int)(ub>>32);
    unsigned int an=(ahi&0x80000000u)?1u:0u,bn=(bhi&0x80000000u)?1u:0u,neg=an^bn;
    if(an){unsigned int nl=(~alo)+1u;unsigned int c=(nl==0u)?1u:0u;ahi=(~ahi)+c;alo=nl;}
    if(bn){unsigned int nl=(~blo)+1u;unsigned int c=(nl==0u)?1u:0u;bhi=(~bhi)+c;blo=nl;}
    unsigned int a[4]={alo&0xFFFFu,alo>>16,ahi&0xFFFFu,ahi>>16};
    unsigned int b[4]={blo&0xFFFFu,blo>>16,bhi&0xFFFFu,bhi>>16};
    unsigned int acc[8]; for(int i=0;i<8;i++)acc[i]=0u;
    for(int i=0;i<4;i++){unsigned int carry=0u;
        for(int j=0;j<4;j++){unsigned int t=a[i]*b[j]+acc[i+j]+carry;acc[i+j]=t&0xFFFFu;carry=t>>16;}
        int k=i+4; while(carry>0u){unsigned int t=acc[k]+carry;acc[k]=t&0xFFFFu;carry=t>>16;k++;}}
    unsigned int w1=acc[2]|(acc[3]<<16),w2=acc[4]|(acc[5]<<16);
    if(neg){unsigned int w0=acc[0]|(acc[1]<<16),w3=acc[6]|(acc[7]<<16);unsigned int c=1u;
        unsigned int v=~w0,s=v+c;c=(s<v)?1u:0u; v=~w1;s=v+c;c=(s<v)?1u:0u;w1=s; v=~w2;s=v+c;c=(s<v)?1u:0u;w2=s; (void)w3;}
    O[idx]=(long long)(((unsigned long long)w2<<32)|(unsigned long long)w1);
}'''
kn = cp.RawKernel(NATIVE, "mul_native", options=("--device-int128",))
kl = cp.RawKernel(LIMB, "mul_limb")

n = 64_000_000
rng = np.random.default_rng(1)
A = rng.integers(-(1<<62), 1<<62, size=n, dtype=np.int64)
B = rng.integers(-(1<<30), 1<<30, size=n, dtype=np.int64)
dA=cp.asarray(A); dB=cp.asarray(B); dO=cp.empty(n, dtype=cp.int64)
threads=256; blocks=(n+threads-1)//threads
def bench(k, reps=20):
    k((blocks,),(threads,),(dA,dB,dO,np.int32(n))); cp.cuda.Device().synchronize()  # warm
    st=cp.cuda.Event(); en=cp.cuda.Event(); st.record()
    for _ in range(reps): k((blocks,),(threads,),(dA,dB,dO,np.int32(n)))
    en.record(); en.synchronize()
    ms=cp.cuda.get_elapsed_time(st,en)/reps
    return n/(ms/1000)/1e6, dO.copy()
mn, on = bench(kn)
ml, ol = bench(kl)
print(f"GPU native  __int128 mul: {mn:,.0f} Mops/s")
print(f"GPU u32-limb mul (portable): {ml:,.0f} Mops/s")
print("native == limb on device:", bool(cp.all(on==ol)))
