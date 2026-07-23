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

//! Canonical GPU compute for the simulator (design Part 5.4, item R-GPU-CANON-PIN), authored in
//! CubeCL: kernels are written in Rust `#[cube]` functions and compiled to the CUDA backend, which is
//! the backend wired and proven here. The one law of this crate is that every canonical kernel is
//! integer-only and reproduces the `crates/core` [`civsim_core::Fixed`] oracle bit-for-bit. It carries
//! no fabricated numbers: a canonical GPU result that disagrees with the CPU oracle by a single bit is
//! a defect, so the crate ships its own Stage 0 bit-identity gate (Part 60) as tests.
//!
//! Two integer-only disciplines coexist here, and the difference matters for the cross-vendor claim.
//! The Stage 0 arithmetic stays inside the u32-limb confined op set (identical defined semantics on
//! every CubeCL target), so its bit-identity is backend-general by the unique-result argument. The
//! field kernel uses native `i64` where the backend has it (the per-kernel layout the proposal leaves
//! to the author): bit-identical on CUDA and proven so, with cross-vendor bit-identity a Stage 0 gate
//! matter for each further backend rather than something already implied. Neither uses a float.
//!
//! Why CubeCL and not the CPU: the pinned integer arithmetic is exact and lane-order-independent, so
//! it is bit-identical across vendors, which is what makes a GPU field kernel usable on the canonical
//! path (Principle 3) rather than as a non-authoritative quantized field. The heavy dependency lives
//! here so the determinism bedrock stays dependency-free; the GPU path is a consumer of the oracle.
//!
//! What is here now (the first stand-up):
//! - [`stage0`]: the pinned Q32.32 sign-magnitude limb multiply and 96-step restoring divide as
//!   `#[cube]` kernels, over the confined op set (u32 wrapping add/sub, u16*u16->u32, bitwise,
//!   constant shifts, comparisons, branchless `select`; no native 64-bit, no divide, no float). These
//!   reproduce `Fixed::mul`/`Fixed::div` bit-for-bit and are the load-bearing arithmetic contract,
//!   checked against independent codegen paths in the sparse CPU and Vulkan evidence lanes. Those
//!   optional backends are not compiled into the ordinary pull-request build.
//! - `prim` (internal): the shared i64-boundary limb primitives, the pinned multiply `q32_mul`, the
//!   divide `q32_div`, and the 96-bit `isqrt_u96`, reproducing `Fixed::mul`/`div`/`sqrt`. The field
//!   stencil and the transcendentals call these rather than each carrying a private copy.
//! - [`field`]: the canonical fixed-point diffusion stencil (the Part 5.5 workload, the GPU
//!   counterpart of `civsim_sim`'s `Field::step` and `crates/core`'s `diffusion_bench`), a real
//!   physics field kernel whose per-cell op sequence is fixed and whose one multiply is the pinned
//!   limb multiply.
//! - [`transcendental`]: the full set (`exp`, `ln`, `powf`, `powi`, `sqrt`, and the CORDIC
//!   `sin`/`cos`/`atan`/`asin`) as `#[cube]` kernels, reproducing the `crates/core` `Fixed` oracle
//!   bit-for-bit by running the same integer algorithm over the limb primitives (a barrel shifter
//!   stands in for the DSL's lack of a runtime-amount shift). The general `sqrt` is the 96-bit limb
//!   `isqrt_u96`; `asin` keeps a domain-limited u64 isqrt over its zero-to-one radicand. These unblock
//!   the transcendental physics laws the waves deferred (Arrhenius, Clausius-Clapeyron, Beer-Lambert,
//!   Nernst, Snell, Rayleigh, and general scaling).
//!
//! Device access is optional: the crate builds with no CUDA present, and the launchers assume a
//! working device only when called. The gate tests self-skip unless `CIVSIM_GPU` is set.

pub mod field;
mod prim;
pub mod stage0;
pub mod transcendental;

pub use field::{gpu_diffuse, gpu_diffuse_tiled, gpu_field_step, gpu_fixed_mul, FieldResident};
#[cfg(feature = "cpu-backend")]
pub use stage0::{cpu_client, CpuClient};
pub use stage0::{
    cuda_client, gpu_div, gpu_div_limb_u32, gpu_div_native_i64, gpu_mul, gpu_mul_limb_u32,
    gpu_mul_native_i64, CudaClient, Stage0Transport,
};
#[cfg(feature = "vulkan-backend")]
pub use stage0::{wgpu_client, wgpu_client_with_adapter_receipt, WgpuAdapterReceipt, WgpuClient};
pub use transcendental::{
    gpu_asin, gpu_atan, gpu_cos, gpu_exp, gpu_ln, gpu_powf, gpu_powi, gpu_sin, gpu_sqrt,
};
