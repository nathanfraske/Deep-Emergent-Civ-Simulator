# Handoff directive: test the `civsim-gpu` build past the tracel-llvm fetch block

**For:** an agent (or session) whose environment can reach the `tracel-ai/tracel-llvm`
GitHub release assets. This session cannot, so the `civsim-gpu` crate could not be
built or tested here. Everything else in the workspace (core, sim, world) builds and
tests green on this branch.

**Branch:** `claude/world-wiring-handoff-t0u76v`.

## What is blocked and why

`cargo build -p civsim-gpu` fails while compiling the build dependency
`tracel-llvm-bundler v20.1.4-7`. That build script downloads a prebuilt LLVM bundle
from `github.com/tracel-ai/tracel-llvm/releases` (it pulls
`linux-x64.checksums.json` first). In this session that request returns HTTP 403 with
the body:

```
{"message":"GitHub access to this repository is not enabled for this session. Use add_repo to request access.", ...}
```

So the failure is an egress policy denial on a third-party repo, not a TLS, proxy, or
CA problem, and not a defect in `civsim-gpu` itself. The session's GitHub egress is
scoped to `nathanfraske/deep-emergent-civ-simulator` only. Per the agent-proxy README,
a 403 policy denial must be reported rather than retried, so the block could not be
worked around from here.

`tracel-llvm-bundler` is pulled in transitively by `cubecl = { version = "0.10",
features = ["cuda", "cpu", "vulkan"] }` (the `cpu` backend uses LLVM). It is a
build-time asset fetch; nothing in `crates/gpu` reads the project's own code path
differently because of it.

Note that `civsim-gpu` is excluded from the workspace `default-members` (see the header
comment in `crates/gpu/Cargo.toml`), so a plain `cargo build` / `cargo test` over the
workspace stays lean and CUDA-free and was never blocked by this. Only an explicit
`-p civsim-gpu` (or `--workspace --all-targets`, which includes it) hits the fetch.

## What to test once the fetch is available

In an environment that can reach `tracel-ai/tracel-llvm` release downloads (for example
one where that repo is added through `add_repo`, or whose network policy allows the
GitHub release asset, or where the LLVM bundle is vendored offline and
`tracel-llvm-bundler` is pointed at it):

1. `cargo build -p civsim-gpu` completes (the bundle downloads and installs to
   `~/.local/share/tracel/tracel-llvm-20.1.4-7/`).
2. `cargo test -p civsim-gpu` passes. The device-touching tests self-skip unless
   `CIVSIM_GPU` is set, so a host with no CUDA device should still see the
   host-side and oracle-gate tests pass. The load-bearing gate is R-GPU-CANON-PIN:
   the `#[cube]` kernels (the pinned Q32.32 limb multiply/divide and the fixed-point
   field stencils) must match the `crates/core` `Fixed` oracle bit for bit, and the
   worldgen gate compares the GPU noise kernel to the CPU oracle `noise::fractal`.
3. With a real device present, set `CIVSIM_GPU=1` and re-run to exercise the
   device-touching path.

Report back whether the crate builds, whether the oracle-gate tests pass bit for bit,
and any version skew (cubecl `0.10` versus the pinned `tracel-llvm 20.1.4-7`).

## Fix paths for the tester's environment

- Add the release source to egress: request `add_repo` for `tracel-ai/tracel-llvm`,
  or run in a session whose network policy permits the GitHub release download.
- Or vendor the LLVM bundle: fetch `linux-x64.checksums.json` and the matching archive
  from the `v20.1.4-7` release once, place them where `tracel-llvm-bundler` expects
  them (it records install state at
  `~/.local/share/tracel/tracel-llvm-20.1.4-7/.tracel-llvm-installed`), and build
  offline.

This is a test-and-verify handoff only. No code change to `civsim-gpu` is implied; the
crate is unchanged on this branch. If the build surfaces a real defect, fix it on this
branch and note it here.


---

## Test results (2026-07-04, on the RTX 5090 box with GPU access)

Verified by the session that holds GPU access. The build is not blocked in this environment: the `tracel-llvm-20.1.4-7` bundle is already installed at `~/.local/share/tracel/` from a prior GPU session, and GitHub egress reaches the release asset (HTTP 302 to the asset, not the 403 the world-wiring session hit), so `tracel-llvm-bundler` skipped the fetch and reused the installed bundle.

Run with `CUDA_PATH=$HOME/.local/cuda` and `LD_LIBRARY_PATH=$HOME/.local/cuda/lib:/usr/lib/wsl/lib`:

1. `cargo build -p civsim-gpu`: BUILDS clean in 5s (cubecl 0.10 and civsim-gpu compile; the bundler found the installed bundle and did not re-fetch).
2. `cargo test -p civsim-gpu` (host-side, device tests self-skip): 25 of 25 pass, 0 failed.
3. `CIVSIM_GPU=1 cargo test -p civsim-gpu` on the RTX 5090 (Blackwell sm_120, driver 595.97, NVRTC 12.9): ALL pass, 0 failed, exit 0. The R-GPU-CANON-PIN oracle gate `stage0_arithmetic_agrees_across_cuda_and_cpu_backends` passes: the CUDA `#[cube]` Q32.32 limb multiply and restoring divide match the `civsim_core::Fixed` oracle bit for bit on the device, and the field-stencil and worldgen-noise device tests pass (9 tests in 7.3s of real NVRTC compile plus device execution).

No version skew: cubecl 0.10 pulls tracel-llvm 20.1.4-7 (the LLVM-backed `cpu` backend) and the CUDA path builds and runs against driver 595.97 on the 5090 without incident. No defect surfaced in `civsim-gpu`; the crate is unchanged.

One note for the blocked-environment problem, not a defect: the `cpu` backend is load-bearing here, since the oracle gate `stage0_arithmetic_agrees_across_cuda_and_cpu_backends` compares the CUDA and CPU (LLVM) backends for cross-backend bit identity, so dropping the `cpu` feature would remove a comparison target of the R-GPU-CANON-PIN gate rather than being a free trim. The `tracel-llvm` dependency is therefore intentional, and the right unblock for a session without egress is the handoff's own fix paths (add_repo for `tracel-ai/tracel-llvm`, or vendoring the bundle). Correction verified after the merge to main: the `vulkan` backend is also load-bearing. `stage0_arithmetic_agrees_on_wgpu_spirv_backend` exercises the wgpu/SPIR-V path in the same cross-vendor bit-identity gate, so all three backends (`cuda`, `cpu`, `vulkan`) are comparison targets of R-GPU-CANON-PIN and none is a free trim. `vulkan` does not pull `tracel-llvm`, so it does not affect the egress block either way.
