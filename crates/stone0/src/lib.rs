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

//! # civsim-stone0: the local-firing provenance gate
//!
//! Stone 0 is the meta-gate that makes the no-fabricated-values discipline un-bypassable at the local
//! inner loop (design `docs/working/Q1_STONE0_PROVENANCE_GATE_DESIGN.md`). This crate is the gate's
//! library and its `stone0-gate` binary. INCREMENT 4 IS DONE and the gate fires at BUILD time:
//! `crates/sim/build.rs` calls `run(Mode::Local)` and panics on a positive detection, so the scan runs on
//! every `cargo build`, `check`, `test`, and `clippy` of `civsim-sim`, and a blocked build stops there
//! with the gate's report. The binary remains the direct entry point for `--ci` and `--self-test`.
//!
//! Said plainly because the previous wording claimed the opposite ("NOT yet wired into any build script"),
//! which would send a developer whose build is blocked here to rule this gate out as the cause when it is
//! the most likely one.
//!
//! ## The checks
//!
//! 1. Provenance scan (the violation predicate): shells out to the existing python gates
//!    (`constructor_gate.py`, `provenance_gate.py`, `floor_provenance_gate.py`, `determinism_gate.py`,
//!    `quarantine_gate.py`) and collects any failure. The verdict is cached by a content hash of the
//!    scanned files plus the scripts, so a repeated run with unchanged inputs is cheap.
//! 2. Live-password laundering scan (local only): if the secrets file exists, reads the password and
//!    scans every git-tracked file plus the git index for the literal password and its base64. A hit is
//!    a hard fail. The password value is never printed, logged, written, or persisted; only the file
//!    where a hit occurred is reported.
//! 3. Tombstone scan: every retired (now-declassified) override phrase in `stone0_tombstones.txt` must
//!    appear nowhere but the tombstone list itself. A hit is a laundered stale copy and fails.
//! 4. Override-env-name scan: a committed file that assigns `STONE0_OVERRIDE` has baked the override in
//!    and fails.
//! 5. Canary: a `CANARY=` line in the secrets file that appears anywhere in the tracked tree proves the
//!    secrets file was read and its contents committed, and fails.
//!
//! ## Robustness contract
//!
//! The gate fails OPEN on any operational error (python missing, secrets dir absent, git unavailable):
//! it prints a warning and allows the build. It fails CLOSED only on a positive detection. A gate bug or
//! a missing dependency must never brick a build.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The default out-of-repo secrets file the owner keeps the override password in.
pub const DEFAULT_SECRETS_PATH: &str = "/mnt/e/Secrets/stone0-override.pass";
/// The committed tombstone list, one retired (declassified) override phrase per line.
pub const TOMBSTONE_REL: &str = "calibration/stone0_tombstones.txt";
/// The environment variable an owner-authorized single-command override is supplied through.
pub const OVERRIDE_ENV: &str = "STONE0_OVERRIDE";

/// The instruction block printed when the provenance scan fails with no valid override.
pub const OVERRIDE_INSTRUCTIONS: &str = "This value has no provenance. To override you must obtain the current password from Nathan (out of band), set STONE0_OVERRIDE for a single command, and Nathan will rotate the password afterward. Do NOT write the password into the repo; CI will catch it via the tombstone list and halt.";

/// The five existing python gates the provenance scan consolidates. A script that is absent is skipped
/// silently (fail-open), so a not-yet-created gate never bricks a build.
/// Every gate Stone 0 runs, with its arguments. An entry is `(script, args)`.
///
/// THE LIST WAS HALF THE GATES. It named five and omitted sources, source generation, derives, the
/// diamond scan, the profile-override check, the floor-registry staleness check and all eight
/// per-source provenance tests. The omission was invisible from Stone 0's own output, because a gate
/// that is not in this list reports nothing at all: the failing Grueneisen witnesses were unseen here
/// for exactly that reason.
///
/// A gate that takes an argument is listed WITH it, because `--strict` and `--check` are where several
/// of these actually convict; running them bare is how the diamond scan passed for months without ever
/// looking at the repository.
const PROVENANCE_SCRIPTS: &[(&str, &[&str])] = &[
    ("scripts/constructor_gate.py", &[]),
    ("scripts/provenance_gate.py", &[]),
    ("scripts/floor_provenance_gate.py", &[]),
    ("scripts/determinism_gate.py", &[]),
    ("scripts/quarantine_gate.py", &[]),
    ("scripts/sources_gate.py", &[]),
    ("scripts/gen_sources.py", &["--check"]),
    ("scripts/derives_gate.py", &[]),
    ("scripts/diamond_gate.py", &["--strict"]),
    ("scripts/profile_override_gate.py", &[]),
    ("scripts/gen_floor_registry.py", &["--check"]),
];

/// Which run this is. `Local` runs every check including the live-password and canary scans against the
/// secrets file. `Ci` runs the provenance, tombstone, and override-env scans only (a hosted runner has
/// no secrets dir and must never attempt the live-password scan). `SelfTest` runs the internal proofs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Local,
    Ci,
    SelfTest,
}

/// The gate's verdict. `Clean` and `Overridden` both allow the build (exit 0); `Fail` blocks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Clean,
    Overridden,
    Fail,
}

/// The full result of a gate run.
#[derive(Debug, Clone)]
pub struct GateReport {
    pub verdict: Verdict,
    /// Positive detections that fail the build closed.
    pub failures: Vec<String>,
    /// Operational problems that fail open (a warning, never a block).
    pub warnings: Vec<String>,
    /// Informational notices (the loud override banner text lives here).
    pub notices: Vec<String>,
}

impl GateReport {
    /// The process exit code: 1 on a hard fail, 0 otherwise.
    pub fn exit_code(&self) -> i32 {
        if self.verdict == Verdict::Fail {
            1
        } else {
            0
        }
    }
}

/// The inputs a gate run keys off. Constructed from the environment by [`run`], or by hand in tests.
pub struct GateConfig {
    pub repo_root: PathBuf,
    pub secrets_path: PathBuf,
    pub tombstones_path: PathBuf,
    pub mode: Mode,
    /// The value supplied through `STONE0_OVERRIDE`, if any (only consulted in `Local` mode).
    pub override_value: Option<String>,
}

/// One tracked file's path and raw bytes. The password scan is byte-exact (it catches a hit even in a
/// binary blob); the text scans read the bytes as UTF-8 lossily.
#[derive(Debug, Clone)]
pub struct RepoFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

impl RepoFile {
    /// Build a file record from any byte content (used by tests).
    pub fn new(path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        RepoFile {
            path: path.into(),
            bytes: bytes.into(),
        }
    }

    fn text(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.bytes)
    }
}

// ----------------------------------------------------------------------------------------------------
// Inline base64 (standard alphabet, with padding). Implemented here so the gate depends on nothing.
// ----------------------------------------------------------------------------------------------------

/// Standard base64 (RFC 4648 alphabet, `=` padded) of the input bytes.
pub fn base64_standard(input: &[u8]) -> String {
    const ALPHA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHA[((n >> 18) & 63) as usize] as char);
        out.push(ALPHA[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHA[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(ALPHA[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

// ----------------------------------------------------------------------------------------------------
// Pure detection functions. Each takes explicit inputs so it is testable without touching the real tree.
// ----------------------------------------------------------------------------------------------------

/// True if `needle` occurs as a contiguous subslice of `hay`. An empty needle never matches.
fn contains_bytes(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Parse the secrets file content into `(password, canary)`. The password is the first non-empty line
/// that does not start with `CANARY=`, trimmed. The canary, if present, is the value after `CANARY=`.
pub fn parse_secrets(content: &str) -> (String, Option<String>) {
    let mut password = String::new();
    let mut canary: Option<String> = None;
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(rest) = t.strip_prefix("CANARY=") {
            if canary.is_none() && !rest.is_empty() {
                canary = Some(rest.to_string());
            }
            continue;
        }
        if password.is_empty() {
            password = t.to_string();
        }
    }
    (password, canary)
}

/// Parse the tombstone list: one retired phrase per non-empty, non-comment line, trimmed.
pub fn parse_tombstones(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
}

/// The files (by path) that contain the live password, its padded base64, or its unpadded base64.
/// An empty password matches nothing. The password value itself is never included in the output.
pub fn find_password_hits(files: &[RepoFile], password: &str) -> Vec<String> {
    if password.is_empty() {
        return Vec::new();
    }
    let lit = password.as_bytes();
    let b64 = base64_standard(password.as_bytes());
    let b64_nopad = b64.trim_end_matches('=').to_string();
    let mut hits = Vec::new();
    for f in files {
        let hit = contains_bytes(&f.bytes, lit)
            || contains_bytes(&f.bytes, b64.as_bytes())
            || (!b64_nopad.is_empty() && contains_bytes(&f.bytes, b64_nopad.as_bytes()));
        if hit {
            hits.push(f.path.clone());
        }
    }
    hits
}

/// The `(path, phrase)` hits where a retired tombstone phrase appears in a file other than the excluded
/// tombstone list itself. The phrase is safe to report: retirement declassifies it.
pub fn find_tombstone_hits(
    files: &[RepoFile],
    tombstones: &[String],
    exclude_exact: &[String],
) -> Vec<(String, String)> {
    let mut hits = Vec::new();
    for f in files {
        if exclude_exact.contains(&f.path) {
            continue;
        }
        let text = f.text();
        for t in tombstones {
            if t.is_empty() {
                continue;
            }
            if text.contains(t.as_str()) {
                hits.push((f.path.clone(), t.clone()));
            }
        }
    }
    hits
}

/// The files that assign the `STONE0_OVERRIDE` environment variable (the name followed, after optional
/// spaces or tabs, by `=` or `:`), excluding any file whose path starts with an excluded prefix.
pub fn find_override_env_hits(files: &[RepoFile], exclude_prefixes: &[String]) -> Vec<String> {
    let mut hits = Vec::new();
    'file: for f in files {
        if exclude_prefixes
            .iter()
            .any(|p| f.path.starts_with(p.as_str()))
        {
            continue;
        }
        let text = f.text();
        for (idx, _) in text.match_indices(OVERRIDE_ENV) {
            let after = &text[idx + OVERRIDE_ENV.len()..];
            let trimmed = after.trim_start_matches([' ', '\t']);
            if trimmed.starts_with('=') || trimmed.starts_with(':') {
                hits.push(f.path.clone());
                continue 'file;
            }
        }
    }
    hits
}

/// The files that contain the secrets-file canary value (excluding any exact-path exclusion). A hit
/// proves the secrets file was read directly and its content committed.
pub fn find_canary_hits(files: &[RepoFile], canary: &str, exclude_exact: &[String]) -> Vec<String> {
    if canary.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for f in files {
        if exclude_exact.contains(&f.path) {
            continue;
        }
        if contains_bytes(&f.bytes, canary.as_bytes()) {
            hits.push(f.path.clone());
        }
    }
    hits
}

/// True if a non-empty override value equals the secrets-file password. Never logs either value.
pub fn override_is_valid(override_value: Option<&str>, password: &str) -> bool {
    match override_value {
        Some(v) => !password.is_empty() && !v.is_empty() && v == password,
        None => false,
    }
}

// ----------------------------------------------------------------------------------------------------
// The provenance scan: shell out to the python gates, with a content-hash verdict cache.
// ----------------------------------------------------------------------------------------------------

/// What one gate run produced.
///
/// `Skipped` USED TO EXIST and is deliberately gone. It absorbed three different situations that are
/// not the same: a script missing from disk, a script that crashed, and an interpreter that could not
/// be spawned. Only the last is operational. Folding the other two into a skip made deleting or
/// breaking a gate the two cheapest ways to stop it convicting, and Stone 0 reported neither.
enum ScriptResult {
    Clean,
    Detected(String),
    Operational(String),
}

struct ProvenanceOutcome {
    detections: Vec<String>,
    operational: Vec<String>,
}

fn run_python_gate(root: &Path, script_rel: &str, args: &[&str]) -> ScriptResult {
    let path = root.join(script_rel);
    if !path.exists() {
        // A LISTED SCRIPT THAT IS ABSENT IS A FAILURE, not a skip. This list names what MUST run, so a
        // missing entry means either the gate was deleted or the path drifted, and both should be loud.
        // Skipping made deleting a gate the quietest way to stop it convicting.
        return ScriptResult::Detected(format!(
            "{script_rel} is listed in PROVENANCE_SCRIPTS but does not exist. A gate that is not there \
             has not passed; restore it or remove its entry deliberately."
        ));
    }
    let out = match Command::new("python3")
        .arg(&path)
        .args(args)
        .current_dir(root)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            return ScriptResult::Operational(format!(
                "could not run python3 for {script_rel} ({e}); skipped"
            ))
        }
    };
    if out.status.success() {
        return ScriptResult::Clean;
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    // A CRASHING GATE HAS NOT PASSED. These three were treated as "operational" and skipped, which made
    // BREAKING a gate the cheapest way to silence it: introduce a syntax error, or an import of
    // something absent, and Stone 0 reported an operational skip and moved on. The one genuinely
    // operational case is python3 itself being unavailable, and that is handled above where the spawn
    // fails; everything reaching here ran the interpreter and the SCRIPT failed.
    if stderr.contains("Traceback (most recent call last)")
        || stderr.contains("ModuleNotFoundError")
        || stderr.contains("SyntaxError")
    {
        let last = stderr.lines().last().unwrap_or("");
        return ScriptResult::Detected(format!(
            "{script_rel} CRASHED ({last}). A gate that cannot run has not passed, and treating this as \
             an operational skip made breaking a gate the cheapest way to silence it."
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    ScriptResult::Detected(format!("{script_rel}:\n{}", stdout.trim_end()))
}

fn provenance_scan(root: &Path) -> ProvenanceOutcome {
    let mut detections = Vec::new();
    let mut operational = Vec::new();
    for (s, args) in PROVENANCE_SCRIPTS {
        match run_python_gate(root, s, args) {
            ScriptResult::Clean => {}
            ScriptResult::Detected(r) => detections.push(r),
            ScriptResult::Operational(w) => operational.push(w),
        }
    }
    ProvenanceOutcome {
        detections,
        operational,
    }
}

fn fnv1a64_update(mut h: u64, data: &[u8]) -> u64 {
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn collect_files_with_ext(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect_files_with_ext(&p, ext, out);
            } else if p.extension().and_then(|e| e.to_str()) == Some(ext) {
                out.push(p);
            }
        }
    }
}

/// A content hash of everything the python gates read, so a verdict can be cached and reused when the
/// inputs are byte-identical. FNV-1a is adequate here: the cache is a local optimization, and CI always
/// runs on a fresh checkout with no cache, so a cache miss is the backstop.
fn provenance_input_hash(root: &Path) -> u64 {
    let mut files: Vec<PathBuf> = Vec::new();
    for (s, _args) in PROVENANCE_SCRIPTS {
        files.push(root.join(s));
    }
    // EVERY INPUT A GATE READS, or a cached clean verdict outlives an edit that would have convicted.
    // The ledger and the profiles were the live gap: editing `quarantine_ledger.toml` locally could reuse
    // a cached pass because the ledger was not hashed and the build script did not declare it as a rerun
    // input, and the calibration profiles are what the simulation actually loads.
    for extra in [
        "scripts/constructor_baseline.tsv",
        "scripts/determinism_baseline.tsv",
        "scripts/derives_baseline.tsv",
        "scripts/profile_override_baseline.tsv",
        "calibration/reserved.toml",
        "calibration/profiles/dev-fixtures.toml",
        "calibration/profiles/mirror.toml",
        "docs/working/quarantine_ledger.toml",
        "docs/working/PHYSICS_FLOOR_REGISTRY.md",
        "sources/registry.toml",
        "sources/mirrored.toml",
    ] {
        files.push(root.join(extra));
    }
    // Every directory the python gates scan must appear here, or an edit inside one of them would not
    // invalidate the cached verdict and a stale verdict would be reused. `crates/bio/src` and
    // `crates/foundation/src` are the scan roots the two crate extractions added (see the CRATES lists
    // in constructor_gate.py and determinism_gate.py); they are covered here for that reason.
    for dir in [
        "crates/core/src",
        "crates/physics/src",
        "crates/bio/src",
        "crates/foundation/src",
        "crates/sim/src",
        "crates/world/src",
    ] {
        collect_files_with_ext(&root.join(dir), "rs", &mut files);
    }
    collect_files_with_ext(&root.join("crates/physics/data"), "toml", &mut files);
    files.sort();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for f in files {
        if let Ok(bytes) = std::fs::read(&f) {
            h = fnv1a64_update(h, f.to_string_lossy().as_bytes());
            h = fnv1a64_update(h, &[0]);
            h = fnv1a64_update(h, &bytes);
        }
    }
    h
}

fn read_cache(cache_path: &Path, want_hash: u64) -> Option<Vec<String>> {
    let text = std::fs::read_to_string(cache_path).ok()?;
    let (hash_line, rest) = text.split_once('\n')?;
    if hash_line.trim() != format!("{want_hash:016x}") {
        return None;
    }
    let dets: Vec<String> = rest
        .split('\u{1e}')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    Some(dets)
}

fn write_cache(cache_path: &Path, hash: u64, detections: &[String]) {
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let body = detections.join("\u{1e}");
    let _ = std::fs::write(cache_path, format!("{hash:016x}\n{body}"));
}

fn provenance_scan_cached(root: &Path, notices: &mut Vec<String>) -> ProvenanceOutcome {
    let hash = provenance_input_hash(root);
    let cache_path = root.join("target/stone0/provenance.cache");
    if let Some(dets) = read_cache(&cache_path, hash) {
        notices.push("provenance verdict served from cache (inputs unchanged)".to_string());
        return ProvenanceOutcome {
            detections: dets,
            operational: Vec::new(),
        };
    }
    let outcome = provenance_scan(root);
    // Only cache when python ran (no operational error), so a transient python outage never
    // freezes a clean verdict into the cache.
    if outcome.operational.is_empty() {
        write_cache(&cache_path, hash, &outcome.detections);
    }
    outcome
}

// ----------------------------------------------------------------------------------------------------
// Tracked-tree and index gathering (git). Every step fails open: any error returns Err and the caller
// warns and skips, never blocks.
// ----------------------------------------------------------------------------------------------------

fn git_tracked_files(root: &Path) -> Result<Vec<RepoFile>, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z"])
        .output()
        .map_err(|e| format!("git ls-files did not spawn ({e})"))?;
    if !out.status.success() {
        return Err("git ls-files exited non-zero".to_string());
    }
    let mut files = Vec::new();
    for rec in out.stdout.split(|&b| b == 0) {
        if rec.is_empty() {
            continue;
        }
        let rel = String::from_utf8_lossy(rec).into_owned();
        match std::fs::read(root.join(&rel)) {
            Ok(bytes) => files.push(RepoFile { path: rel, bytes }),
            Err(_) => {
                // A tracked path with no readable working-tree file (a submodule gitlink, a deleted but
                // still-tracked path). Skip it; the index scan still covers its staged blob.
            }
        }
    }
    Ok(files)
}

/// Read the staged blob content of every tracked path via a single `git cat-file --batch`. The blob
/// hashes are fed on the child's stdin (a writer thread avoids a pipe deadlock); no secret ever touches
/// a command line. Paths are prefixed `(index)` so a report distinguishes a staged hit from a worktree
/// hit.
fn git_index_blobs(root: &Path) -> Result<Vec<RepoFile>, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-s", "-z"])
        .output()
        .map_err(|e| format!("git ls-files -s did not spawn ({e})"))?;
    if !out.status.success() {
        return Err("git ls-files -s exited non-zero".to_string());
    }
    let mut sha_to_path: BTreeMap<String, String> = BTreeMap::new();
    for rec in out.stdout.split(|&b| b == 0) {
        if rec.is_empty() {
            continue;
        }
        let s = String::from_utf8_lossy(rec);
        if let Some((meta, path)) = s.split_once('\t') {
            let parts: Vec<&str> = meta.split_whitespace().collect();
            if parts.len() >= 2 {
                sha_to_path
                    .entry(parts[1].to_string())
                    .or_insert_with(|| path.to_string());
            }
        }
    }
    if sha_to_path.is_empty() {
        return Ok(Vec::new());
    }
    let shas: Vec<String> = sha_to_path.keys().cloned().collect();
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("git cat-file did not spawn ({e})"))?;
    let mut stdin = child.stdin.take().ok_or("git cat-file stdin unavailable")?;
    let writer = std::thread::spawn(move || {
        for sha in shas {
            if writeln!(stdin, "{sha}").is_err() {
                break;
            }
        }
    });
    let mut stdout = child
        .stdout
        .take()
        .ok_or("git cat-file stdout unavailable")?;
    let mut buf = Vec::new();
    stdout
        .read_to_end(&mut buf)
        .map_err(|e| format!("reading git cat-file ({e})"))?;
    let _ = writer.join();
    let _ = child.wait();

    let mut files = Vec::new();
    let mut i = 0usize;
    while i < buf.len() {
        let Some(off) = buf[i..].iter().position(|&b| b == b'\n') else {
            break;
        };
        let nl = i + off;
        let header = String::from_utf8_lossy(&buf[i..nl]).into_owned();
        i = nl + 1;
        let parts: Vec<&str> = header.split(' ').collect();
        if parts.len() == 2 && parts[1] == "missing" {
            continue;
        }
        if parts.len() < 3 {
            continue;
        }
        let size: usize = match parts[2].trim().parse() {
            Ok(n) => n,
            Err(_) => break,
        };
        if i + size > buf.len() {
            break;
        }
        let content = buf[i..i + size].to_vec();
        i += size;
        if i < buf.len() && buf[i] == b'\n' {
            i += 1;
        }
        if let Some(path) = sha_to_path.get(parts[0]) {
            files.push(RepoFile {
                path: format!("(index) {path}"),
                bytes: content,
            });
        }
    }
    Ok(files)
}

// ----------------------------------------------------------------------------------------------------
// The driver.
// ----------------------------------------------------------------------------------------------------

fn override_env_excludes() -> Vec<String> {
    vec![
        // The gate's own source and scripts, and the design doc, legitimately name the variable.
        "crates/stone0/".to_string(),
        "scripts/stone0".to_string(),
        "docs/working/Q1_STONE0_PROVENANCE_GATE_DESIGN.md".to_string(),
    ]
}

fn read_secrets(cfg: &GateConfig, notices: &mut Vec<String>) -> (Option<String>, Option<String>) {
    if cfg.mode != Mode::Local {
        return (None, None);
    }
    match std::fs::read_to_string(&cfg.secrets_path) {
        Ok(content) => {
            let (pw, canary) = parse_secrets(&content);
            let pw = if pw.is_empty() { None } else { Some(pw) };
            (pw, canary)
        }
        Err(_) => {
            notices.push(format!(
                "secrets file {} not present; the live-password, canary, and override checks are skipped (the owner creates it out of band)",
                cfg.secrets_path.display()
            ));
            (None, None)
        }
    }
}

fn read_tombstones(cfg: &GateConfig, warnings: &mut Vec<String>) -> Vec<String> {
    match std::fs::read_to_string(&cfg.tombstones_path) {
        Ok(content) => parse_tombstones(&content),
        Err(_) => {
            warnings.push(format!(
                "tombstone list {} not present; the tombstone scan is skipped",
                cfg.tombstones_path.display()
            ));
            Vec::new()
        }
    }
}

fn dedup(mut v: Vec<String>) -> Vec<String> {
    let set: BTreeSet<String> = v.drain(..).collect();
    set.into_iter().collect()
}

/// Run every Stone 0 check for the given configuration and return the verdict. Never panics; every
/// operational failure degrades to a warning and an allowed build.
pub fn gate(cfg: &GateConfig) -> GateReport {
    let mut failures: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut notices: Vec<String> = Vec::new();

    // Gather the tracked tree once; the laundering, tombstone, override-env, and canary scans share it.
    let files = match git_tracked_files(&cfg.repo_root) {
        Ok(f) => Some(f),
        Err(e) => {
            warnings.push(format!(
                "git tracked-file scan unavailable ({e}); the laundering, tombstone, override-env, and canary scans are skipped"
            ));
            None
        }
    };

    // Check 1: the provenance scan (both modes).
    let prov = provenance_scan_cached(&cfg.repo_root, &mut notices);
    for w in prov.operational {
        warnings.push(w);
    }
    let provenance_failed = !prov.detections.is_empty();

    // Secrets (local mode only).
    let (password, canary) = read_secrets(cfg, &mut notices);

    // Check 4: the override-env-name scan (both modes).
    if let Some(files) = &files {
        for h in find_override_env_hits(files, &override_env_excludes()) {
            failures.push(format!(
                "override-env baked into a committed file: {h} assigns {OVERRIDE_ENV}. The override is a single-command environment variable, never committed; remove the assignment."
            ));
        }
    }

    // Check 3: the tombstone scan (both modes).
    if let Some(files) = &files {
        let tombs = read_tombstones(cfg, &mut warnings);
        if !tombs.is_empty() {
            let exclude = vec![TOMBSTONE_REL.to_string()];
            for (path, _phrase) in find_tombstone_hits(files, &tombs, &exclude) {
                failures.push(format!(
                    "laundered retired override phrase found in {path}: a tombstoned phrase must appear nowhere but the tombstone list. A retired-but-present copy is a stale laundered credential; remove it."
                ));
            }
        }
    }

    // Checks 2 and 5: the live-password laundering scan and the canary (local mode only).
    if cfg.mode == Mode::Local {
        if let (Some(files), Some(pw)) = (&files, password.as_ref()) {
            let mut hits = find_password_hits(files, pw);
            match git_index_blobs(&cfg.repo_root) {
                Ok(idx) => hits.extend(find_password_hits(&idx, pw)),
                Err(e) => warnings.push(format!(
                    "git index scan unavailable ({e}); the git index was not scanned for the live password"
                )),
            }
            for h in dedup(hits) {
                failures.push(format!(
                    "laundered credential: the live override password appears in {h}. The password value is not printed. Remove it immediately, and Nathan must rotate the password."
                ));
            }
        }
        if let (Some(files), Some(can)) = (&files, canary.as_ref()) {
            for h in find_canary_hits(files, can, &[]) {
                failures.push(format!(
                    "canary tripped: the secrets-file canary appears in {h}, proof the secrets file was read and its content committed. Rotate the canary and investigate."
                ));
            }
        }
    }

    // Check 1 verdict, with the override path (local mode only).
    let mut overridden = false;
    if provenance_failed {
        let mut report =
            String::from("provenance scan found un-provenanced or fixture value(s):\n");
        for d in &prov.detections {
            report.push_str(d);
            report.push('\n');
        }
        let valid_override = cfg.mode == Mode::Local
            && override_is_valid(
                cfg.override_value.as_deref(),
                password.as_deref().unwrap_or(""),
            );
        if valid_override {
            overridden = true;
            notices.push("STONE 0 OVERRIDDEN BY OWNER PASSWORD".to_string());
            notices.push(report);
        } else {
            failures.push(format!("{report}\n{OVERRIDE_INSTRUCTIONS}"));
        }
    } else if cfg.mode == Mode::Local && cfg.override_value.is_some() {
        notices.push(
            "note: STONE0_OVERRIDE was set but the provenance scan was clean; the override was unused"
                .to_string(),
        );
    }

    let verdict = if !failures.is_empty() {
        Verdict::Fail
    } else if overridden {
        Verdict::Overridden
    } else {
        Verdict::Clean
    };

    GateReport {
        verdict,
        failures,
        warnings,
        notices,
    }
}

// ----------------------------------------------------------------------------------------------------
// The binary entry point.
// ----------------------------------------------------------------------------------------------------

fn detect_repo_root() -> PathBuf {
    if let Ok(out) = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return PathBuf::from(s);
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn emit_report(report: &GateReport) {
    for w in &report.warnings {
        eprintln!("stone0: warning: {w}");
    }
    for n in &report.notices {
        if n == "STONE 0 OVERRIDDEN BY OWNER PASSWORD" {
            eprintln!("\n================================================================");
            eprintln!("  STONE 0 OVERRIDDEN BY OWNER PASSWORD");
            eprintln!("  The provenance gate found a violation, but a valid owner");
            eprintln!("  override was supplied. Allowing this one build.");
            eprintln!("================================================================\n");
        } else {
            eprintln!("stone0: {n}");
        }
    }
    for f in &report.failures {
        eprintln!("stone0: FAIL: {f}");
    }
    match report.verdict {
        Verdict::Clean => eprintln!("stone0: clean (every check passed)"),
        Verdict::Overridden => {
            eprintln!("stone0: allowed by owner override (the provenance violation stands, recorded above)")
        }
        Verdict::Fail => {
            eprintln!("stone0: BLOCKED (a positive detection; see the failures above)")
        }
    }
}

/// The binary entry point: build the config from the environment, run the gate, print the report, and
/// return the process exit code.
pub fn run(mode: Mode) -> i32 {
    if mode == Mode::SelfTest {
        return self_test();
    }
    let repo_root = detect_repo_root();
    let secrets_path = std::env::var("STONE0_SECRETS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_SECRETS_PATH));
    let tombstones_path = repo_root.join(TOMBSTONE_REL);
    let override_value = if mode == Mode::Local {
        std::env::var(OVERRIDE_ENV).ok()
    } else {
        None
    };
    let cfg = GateConfig {
        repo_root,
        secrets_path,
        tombstones_path,
        mode,
        override_value,
    };
    let report = gate(&cfg);
    emit_report(&report);
    report.exit_code()
}

// ----------------------------------------------------------------------------------------------------
// The self-test: prove each detector fires (and does not misfire) on synthetic fixtures. Mirrors the
// `--self-test` convention of the python gates.
// ----------------------------------------------------------------------------------------------------

fn self_test() -> i32 {
    let mut problems: Vec<String> = Vec::new();
    let mut check = |name: &str, ok: bool| {
        if !ok {
            problems.push(name.to_string());
        }
    };

    // base64 known vectors (RFC 4648).
    check("base64 of empty", base64_standard(b"").is_empty());
    check("base64 of 'f'", base64_standard(b"f") == "Zg==");
    check("base64 of 'fo'", base64_standard(b"fo") == "Zm8=");
    check("base64 of 'hello'", base64_standard(b"hello") == "aGVsbG8=");

    // override comparison accepts the right password, rejects a wrong one, an absent one, an empty one.
    check(
        "override accepts match",
        override_is_valid(Some("s3cret pass"), "s3cret pass"),
    );
    check(
        "override rejects mismatch",
        !override_is_valid(Some("wrong"), "s3cret pass"),
    );
    check(
        "override rejects absent",
        !override_is_valid(None, "s3cret pass"),
    );
    check(
        "override rejects empty password",
        !override_is_valid(Some(""), ""),
    );

    // password laundering: literal and base64 detection.
    let pw = "correct horse";
    let b64 = base64_standard(pw.as_bytes());
    let files = vec![
        RepoFile::new("clean.txt", b"nothing to see".to_vec()),
        RepoFile::new("literal.sh", format!("PASS={pw}\n").into_bytes()),
        RepoFile::new("encoded.txt", format!("token: {b64}").into_bytes()),
    ];
    let hits = find_password_hits(&files, pw);
    check(
        "password literal caught",
        hits.contains(&"literal.sh".to_string()),
    );
    check(
        "password base64 caught",
        hits.contains(&"encoded.txt".to_string()),
    );
    check(
        "password clean file not flagged",
        !hits.contains(&"clean.txt".to_string()),
    );
    check(
        "empty password matches nothing",
        find_password_hits(&files, "").is_empty(),
    );

    // tombstone detection, with the tombstone file itself excluded.
    let tombs = vec!["retired phrase alpha".to_string()];
    let tfiles = vec![
        RepoFile::new(
            "calibration/stone0_tombstones.txt",
            b"retired phrase alpha\n".to_vec(),
        ),
        RepoFile::new(
            "notes.md",
            b"someone wrote retired phrase alpha here".to_vec(),
        ),
    ];
    let thits = find_tombstone_hits(&tfiles, &tombs, &[TOMBSTONE_REL.to_string()]);
    check(
        "tombstone hit caught in other file",
        thits.iter().any(|(p, _)| p == "notes.md"),
    );
    check(
        "tombstone list itself not flagged",
        !thits
            .iter()
            .any(|(p, _)| p == "calibration/stone0_tombstones.txt"),
    );

    // override-env-name detection across shell, env, and yaml forms; prose is not flagged.
    let ofiles = vec![
        RepoFile::new("a.sh", b"export STONE0_OVERRIDE=hunter2\n".to_vec()),
        RepoFile::new("b.env", b"STONE0_OVERRIDE = value\n".to_vec()),
        RepoFile::new("c.yml", b"  STONE0_OVERRIDE: value\n".to_vec()),
        RepoFile::new(
            "prose.md",
            b"the STONE0_OVERRIDE variable is supplied out of band".to_vec(),
        ),
    ];
    let ohits = find_override_env_hits(&ofiles, &[]);
    check(
        "override-env shell caught",
        ohits.contains(&"a.sh".to_string()),
    );
    check(
        "override-env spaced caught",
        ohits.contains(&"b.env".to_string()),
    );
    check(
        "override-env yaml caught",
        ohits.contains(&"c.yml".to_string()),
    );
    check(
        "override-env prose not flagged",
        !ohits.contains(&"prose.md".to_string()),
    );
    let excluded = find_override_env_hits(&ofiles, &["a.sh".to_string()]);
    check(
        "override-env exclusion works",
        !excluded.contains(&"a.sh".to_string()),
    );

    // canary detection.
    let cfiles = vec![RepoFile::new(
        "leak.log",
        b"debug dump CANARY-XYZ-123 end".to_vec(),
    )];
    let chits = find_canary_hits(&cfiles, "CANARY-XYZ-123", &[]);
    check("canary caught", chits.contains(&"leak.log".to_string()));
    check(
        "empty canary matches nothing",
        find_canary_hits(&cfiles, "", &[]).is_empty(),
    );

    // secrets parsing: password is the first non-CANARY line, the canary is read separately.
    let (spw, scan) = parse_secrets("  my pass phrase  \nCANARY=abc123\n");
    check("secrets password parsed", spw == "my pass phrase");
    check("secrets canary parsed", scan.as_deref() == Some("abc123"));

    if problems.is_empty() {
        println!("stone0 gate self-test: PASS (every detector fires and none misfires)");
        0
    } else {
        println!("stone0 gate self-test: FAIL");
        for p in &problems {
            println!("  - {p}");
        }
        1
    }
}
