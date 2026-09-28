# Fleet execution

Owner pushes, scheduled runs and manual CI runs use the fleet's Linux runners.
Pull requests and forks use GitHub-hosted runners. The workflow's existing
document, format, lint, doc-link, provenance, Stone 0 and test gates are retained,
including the existing distinction between the quick and full test suites.

The job container is the official Rust 1.94.1 Bookworm image pinned by digest.
`rust-toolchain.toml` remains authoritative. The only additional native packages
are minifb's documented Linux prerequisites, installed by
`scripts/setup-fleet-container.sh`. That recipe executes in the job container.
The source and job environment are discarded after the ephemeral runner finishes.

Fleet Rust toolchains, Cargo downloads and build outputs, and XDG download caches
survive under a repository-specific directory on each machine's cache disk.
Docker image layers also survive. The existing shared LAN cache serves npm
packages; it does not yet serve this Rust project's crates or compiler outputs.
The workflow bounds compilation and test concurrency to available memory and
the laptop's current resource allocation. Fleet placement continues to rank the
faster available eligible machine, with other Linux hosts providing overflow.

Independent scientific jobs, such as separate test fixtures or seed sweeps, can
use a job matrix and write results as Actions artifacts. A single simulation
spanning machines requires explicit distribution in the application; these
runners do not combine host RAM or GPU memory into one address space. Preserve
the project's existing calibration rules when adding scientific campaigns.

The full CPU fleet retest [36468661142](https://github.com/nathanfraske/Deep-Emergent-Civ-Simulator/actions/runs/36468661142)
passed all four jobs, including the slow suite, at commit
`949269753c0f6e6ca2ff31c2d3972d018457fee6`. The permanent Linux runner and
optional laptop served the jobs. Earlier fleet failures are retained separately.

The dedicated `fleet-gpu.yml` workflow requests
`[self-hosted, Linux, X64, fleet-gpu-rtx5070]`. Its native WSL runner uses a
6-CPU, 10-GiB job limit, a disposable OS clone and persistent caches. The check
runs a CUDA kernel through the driver API, compares every output with an
independent CPU calculation, checks an overrun guard and retains the physical
GPU UUID, source hash and GitHub execution identity. It needs no CUDA toolkit.
Run [36479431435](https://github.com/nathanfraske/Deep-Emergent-Civ-Simulator/actions/runs/36479431435)
passed the numerical check at `e6f2ed8f904f91c966825b1004e690fae39290bf`.
The controller then removed the ephemeral runner and reset its environment;
the fresh clone was stopped, its cache intact and its work directory empty.
This profile now accepts this repository's owner push and manual GPU jobs.

The simulator's `CIVSIM_GPU` tests remain a separate qualification. The driver
check does not prove CubeCL or NVRTC compatibility and ordinary CI does not set
`CIVSIM_GPU`.

The `fleet-b70.yml` workflow exposes a manual choice of `b70-a`, `b70-b`,
`b70-pair` or `all`. Its qualification remains in progress. The two single-card
jobs can overlap when the workstation has capacity. A paired job reserves both
cards and waits for either single-card reservation to retire.

| Runner label | Devices | Job allocation | Available runtime |
| --- | --- | --- | --- |
| `fleet-gpu-rtx5070` | RTX 5070 | 6 CPUs, 10 GiB RAM | CUDA driver API, Python |
| `fleet-gpu-b70-a` | B70 at `0000:04:00.0` | 8 CPUs, 32 GiB RAM | Intel oneAPI 2026.1, SYCL/Level Zero, `icpx` |
| `fleet-gpu-b70-b` | B70 at `0000:08:00.0` | 8 CPUs, 32 GiB RAM | Intel oneAPI 2026.1, SYCL/Level Zero, `icpx` |
| `fleet-gpu-b70-pair` | Both B70s | 16 CPUs, 64 GiB RAM | Intel oneAPI 2026.1, SYCL/Level Zero, `icpx` |

Use `[self-hosted, Linux, X64, <runner-label>]` in the corresponding admitted
GPU workflow. The controller creates these runners on demand, so an idle
machine need not appear as a permanently registered GitHub runner. Ordinary
CPU labels do not request a GPU. GPU jobs run natively in the isolated job
environment. The B70 compiler and runtime are already on `PATH`; the selected
profile, device count and PCI addresses are exposed as `FLEET_GPU_PROFILE`,
`FLEET_GPU_COUNT` and `FLEET_GPU_PCI`. The integrated GPU is excluded.

The B70 example compiles a SYCL kernel, verifies every integer output against
a CPU reference, checks guard values and records the physical PCI identity of
each device. Paired mode submits work to both devices. It does not combine
their VRAM automatically. Application code must distribute work explicitly.
Per-profile compiler/download caches persist; job work directories and runner
credentials are disposable. Save scientific results with Actions artifacts.
Additional compute frameworks and the simulator's GPU backend need their own
dependency recipe and numerical qualification.

Rollback: restore the prior workflow and remove this repository's exact workflow
entry from the controller's native routing and optional-laptop allowlists.
Existing repository routes, caches and machine resource reservations are retained.
