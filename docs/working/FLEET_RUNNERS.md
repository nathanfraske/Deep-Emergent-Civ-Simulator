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

GPU qualification is separate. The current fleet CI does not set `CIVSIM_GPU`;
CUDA device tests therefore retain their existing opt-in behavior. On 28
September 2026, the RTX 5070 on DESKTOP-BISE755 had a working host driver but
its Hyper-V runner had neither a GPU partition nor a passed-through device.
No CUDA runner label is advertised. Enabling it requires a dedicated GPU-capable
execution environment and a real `CIVSIM_GPU=1` comparison against the CPU
oracle. A CPU CI pass does not establish GPU readiness.

Rollback: restore the prior workflow and remove this repository's exact workflow
entry from the controller's native routing and optional-laptop allowlists.
Existing repository routes, caches and machine resource reservations are retained.
