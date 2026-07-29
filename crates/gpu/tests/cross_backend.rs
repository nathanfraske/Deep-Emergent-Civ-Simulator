// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The cross-backend determinism gate (R-GPU-CANON-PIN, the multi-vendor Stage 0 residual). The
//! canonical Stage 0 arithmetic (the pinned multiply and divide) is run on two independent CubeCL
//! backends, CUDA (NVRTC-compiled) and the CPU backend (MLIR/LLVM-compiled), and each output is
//! checked against the `crates/core` `Fixed` oracle. Agreement of both with the oracle means they
//! agree with each other: two independent integer codegen paths produce bit-identical output, the
//! evidence the "only CUDA is proven" note wanted. Self-skips unless `CIVSIM_GPU` is set (it uses the
//! CUDA client, so it needs a device present).
//!
//! Scope limit: the transcendental kernels (`exp` and its siblings) do NOT yet run on the CPU backend.
//! Their more complex control flow (the barrel shifter, the manually unrolled series) trips a panic in
//! `cubecl-cpu`'s constant-propagation pass (`cubecl-opt`), which appears to be an upstream cubecl
//! optimizer bug rather than a kernel defect: the panic is inside cubecl's own pass, and the CUDA
//! backend proves the transcendentals bit-identical (`transcendental_gate.rs`). It is not filed
//! upstream or reduced to a reproducer here, so "appears" is the honest word.
//! A third codegen path, the wgpu Vulkan/SPIR-V backend (cubecl-spirv), is gated separately below and
//! also reproduces the arithmetic bit-for-bit. Its transcendental confirmation is blocked only by this
//! box's hardware: the sole Vulkan adapter is lavapipe (software), too slow for the large unrolled
//! transcendental shaders, though SPIR-V Int64 is supported (a native-i64 probe ran in milliseconds),
//! so a hardware Vulkan device would carry them. The transcendental cross-backend thus awaits either a
//! cubecl-cpu optimizer fix or a hardware Vulkan ICD; this gate covers the load-bearing arithmetic.

#[cfg(any(feature = "cpu-backend", feature = "vulkan-backend"))]
use civsim_core::Fixed;
#[cfg(feature = "vulkan-backend")]
use civsim_gpu::wgpu_client_with_adapter_receipt;
#[cfg(feature = "cpu-backend")]
use civsim_gpu::{cpu_client, cuda_client};
#[cfg(any(feature = "cpu-backend", feature = "vulkan-backend"))]
use civsim_gpu::{gpu_div, gpu_mul};

#[cfg(feature = "cpu-backend")]
fn gpu_enabled() -> bool {
    std::env::var("CIVSIM_GPU").is_ok()
}

#[cfg(any(feature = "cpu-backend", feature = "vulkan-backend"))]
fn corners() -> Vec<i64> {
    vec![
        0,
        1,
        -1,
        2,
        -2,
        i64::MAX,
        i64::MIN,
        i64::MIN + 1,
        i64::MAX - 1,
        1 << 32,
        -(1 << 32),
        (1 << 32) + 1,
        1 << 31,
        -(1 << 31),
        1i64 << 62,
        -(1i64 << 62),
    ]
}

/// Corner pairs plus a modest deterministic sweep (kept small: the CPU backend JIT-compiles and runs
/// on the host, so it is far slower than CUDA).
#[cfg(any(feature = "cpu-backend", feature = "vulkan-backend"))]
fn cases(seed: u64, sweep: usize) -> (Vec<i64>, Vec<i64>) {
    let cs = corners();
    let mut a = Vec::new();
    let mut b = Vec::new();
    for &x in &cs {
        for &y in &cs {
            a.push(x);
            b.push(y);
        }
    }
    let mut s = seed;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s as i64
    };
    for _ in 0..sweep {
        a.push(next());
        b.push(next());
    }
    (a, b)
}

#[test]
#[cfg(feature = "cpu-backend")]
fn stage0_arithmetic_agrees_across_cuda_and_cpu_backends() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping cross-backend gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let cuda = cuda_client();
    let cpu = cpu_client();

    // Multiply.
    let (a, b) = cases(0x1234_5678_9ABC_DEF1, 20_000);
    let m_cuda = gpu_mul(&cuda, &a, &b);
    let m_cpu = gpu_mul(&cpu, &a, &b);
    let mut mism = 0u64;
    for i in 0..a.len() {
        let oracle = Fixed::from_bits(a[i]).mul(Fixed::from_bits(b[i])).to_bits();
        if m_cuda[i] != oracle || m_cpu[i] != oracle {
            mism += 1;
            if mism <= 5 {
                eprintln!(
                    "mul a={:#x} b={:#x} cuda={:#x} cpu={:#x} oracle={:#x}",
                    a[i], b[i], m_cuda[i], m_cpu[i], oracle
                );
            }
        }
    }
    assert_eq!(
        mism,
        0,
        "mul: CUDA and CPU backends must both equal the oracle over {} cases",
        a.len()
    );

    // Divide (drop zero divisors).
    let (da, db) = cases(0x0FED_CBA9_8765_4321, 20_000);
    let mut ka = Vec::new();
    let mut kb = Vec::new();
    for i in 0..da.len() {
        if db[i] != 0 {
            ka.push(da[i]);
            kb.push(db[i]);
        }
    }
    let d_cuda = gpu_div(&cuda, &ka, &kb);
    let d_cpu = gpu_div(&cpu, &ka, &kb);
    let mut dmism = 0u64;
    for i in 0..ka.len() {
        let oracle = Fixed::from_bits(ka[i])
            .div(Fixed::from_bits(kb[i]))
            .to_bits();
        if d_cuda[i] != oracle || d_cpu[i] != oracle {
            dmism += 1;
        }
    }
    assert_eq!(
        dmism,
        0,
        "div: CUDA and CPU backends must both equal the oracle over {} cases",
        ka.len()
    );
}

/// Opt-in for the wgpu/SPIR-V gate. Unlike the CUDA gate it needs no CUDA device, but it needs a
/// Vulkan adapter (lavapipe suffices), so it is off by default and enabled with `CIVSIM_GPU_WGPU=1`.
#[cfg(feature = "vulkan-backend")]
fn wgpu_enabled() -> bool {
    std::env::var("CIVSIM_GPU_WGPU").is_ok()
}

#[test]
#[cfg(feature = "vulkan-backend")]
fn stage0_arithmetic_agrees_on_wgpu_spirv_backend() {
    if !wgpu_enabled() {
        eprintln!(
            "civsim-gpu: skipping wgpu/SPIR-V gate (set CIVSIM_GPU_WGPU=1; needs a Vulkan adapter)"
        );
        return;
    }
    let (wgpu, adapter) = wgpu_client_with_adapter_receipt();
    println!("{}", adapter.canonical_text());
    assert_eq!(adapter.backend, "Vulkan", "evidence must use Vulkan");
    if std::env::var("CIVSIM_REQUIRE_HARDWARE_GPU").is_ok() {
        assert!(
            !adapter.is_software_adapter(),
            "hardware evidence may not use a software adapter: {adapter:?}"
        );
    }
    if let Ok(expected_vendor) = std::env::var("CIVSIM_EXPECTED_GPU_VENDOR") {
        assert!(
            adapter.matches_vendor(&expected_vendor),
            "selected adapter does not match vendor {expected_vendor}: {adapter:?}"
        );
    }

    // Multiply: a third independent codegen path (SPIR-V) must match the oracle bit-for-bit.
    let (a, b) = cases(0x1234_5678_9ABC_DEF1, 5_000);
    let m = gpu_mul(&wgpu, &a, &b);
    let mut mism = 0u64;
    for i in 0..a.len() {
        let oracle = Fixed::from_bits(a[i]).mul(Fixed::from_bits(b[i])).to_bits();
        if m[i] != oracle {
            mism += 1;
            if mism <= 5 {
                eprintln!(
                    "wgpu mul a={:#x} b={:#x} got={:#x} want={:#x}",
                    a[i], b[i], m[i], oracle
                );
            }
        }
    }
    assert_eq!(
        mism,
        0,
        "wgpu/SPIR-V mul must equal the oracle over {} cases",
        a.len()
    );

    // Divide (drop zero divisors).
    let (da, db) = cases(0x0FED_CBA9_8765_4321, 5_000);
    let mut ka = Vec::new();
    let mut kb = Vec::new();
    for i in 0..da.len() {
        if db[i] != 0 {
            ka.push(da[i]);
            kb.push(db[i]);
        }
    }
    let d = gpu_div(&wgpu, &ka, &kb);
    let mut dmism = 0u64;
    for i in 0..ka.len() {
        let oracle = Fixed::from_bits(ka[i])
            .div(Fixed::from_bits(kb[i]))
            .to_bits();
        if d[i] != oracle {
            dmism += 1;
        }
    }
    assert_eq!(
        dmism,
        0,
        "wgpu/SPIR-V div must equal the oracle over {} cases",
        ka.len()
    );
}
