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

//! Integration tests for the Stone 0 gate. The positive-detection tests use in-memory fixtures so a real
//! secret is never planted in the repo. One test runs the native scans against the real worktree and
//! asserts no false positive.

use std::path::PathBuf;
use std::process::Command;

use civsim_stone0::{
    base64_standard, find_canary_hits, find_override_env_hits, find_password_hits,
    find_tombstone_hits, gate, override_is_valid, parse_secrets, parse_tombstones, GateConfig,
    Mode, RepoFile, Verdict, TOMBSTONE_REL,
};

#[test]
fn base64_matches_rfc4648_vectors() {
    assert_eq!(base64_standard(b""), "");
    assert_eq!(base64_standard(b"f"), "Zg==");
    assert_eq!(base64_standard(b"fo"), "Zm8=");
    assert_eq!(base64_standard(b"foo"), "Zm9v");
    assert_eq!(base64_standard(b"foob"), "Zm9vYg==");
    assert_eq!(base64_standard(b"fooba"), "Zm9vYmE=");
    assert_eq!(base64_standard(b"foobar"), "Zm9vYmFy");
}

#[test]
fn override_accepts_the_right_password_and_rejects_a_wrong_one() {
    assert!(override_is_valid(
        Some("a memorable pass"),
        "a memorable pass"
    ));
    assert!(!override_is_valid(
        Some("almost the pass"),
        "a memorable pass"
    ));
    assert!(!override_is_valid(None, "a memorable pass"));
    assert!(!override_is_valid(Some(""), ""));
    assert!(!override_is_valid(Some("anything"), ""));
}

#[test]
fn password_laundering_is_caught_literal_and_base64() {
    let pw = "purple monkey dishwasher";
    let encoded = base64_standard(pw.as_bytes());
    let files = vec![
        RepoFile::new("src/clean.rs", b"// nothing sensitive here".to_vec()),
        RepoFile::new("scripts/leak.sh", format!("OVERRIDE={pw}").into_bytes()),
        RepoFile::new("data/blob.txt", format!("x{encoded}y").into_bytes()),
    ];
    let hits = find_password_hits(&files, pw);
    assert!(hits.contains(&"scripts/leak.sh".to_string()));
    assert!(hits.contains(&"data/blob.txt".to_string()));
    assert!(!hits.contains(&"src/clean.rs".to_string()));
    // The reported hit is a path, never the password value.
    assert!(hits.iter().all(|h| !h.contains(pw)));
}

#[test]
fn a_planted_tombstone_is_caught_and_the_list_itself_is_not() {
    let tombs = parse_tombstones("# retired phrases\nold pass one\nold pass two\n");
    assert_eq!(tombs, vec!["old pass one", "old pass two"]);
    let files = vec![
        RepoFile::new(TOMBSTONE_REL, b"old pass one\nold pass two\n".to_vec()),
        RepoFile::new("docs/notes.md", b"I stashed old pass two in here".to_vec()),
    ];
    let hits = find_tombstone_hits(&files, &tombs, &[TOMBSTONE_REL.to_string()]);
    assert!(hits
        .iter()
        .any(|(p, phrase)| p == "docs/notes.md" && phrase == "old pass two"));
    assert!(!hits.iter().any(|(p, _)| p == TOMBSTONE_REL));
}

#[test]
fn a_planted_override_env_assignment_is_caught() {
    let files = vec![
        RepoFile::new(
            "ci/run.sh",
            b"STONE0_OVERRIDE=letmein cargo build\n".to_vec(),
        ),
        RepoFile::new("readme.md", b"never set STONE0_OVERRIDE in a file".to_vec()),
    ];
    let hits = find_override_env_hits(&files, &[]);
    assert!(hits.contains(&"ci/run.sh".to_string()));
    assert!(!hits.contains(&"readme.md".to_string()));
}

#[test]
fn the_canary_is_caught_when_committed() {
    let files = vec![RepoFile::new("build.log", b"...CANARY-777... end".to_vec())];
    assert!(find_canary_hits(&files, "CANARY-777", &[]).contains(&"build.log".to_string()));
    assert!(find_canary_hits(&files, "", &[]).is_empty());
}

#[test]
fn secrets_parsing_separates_password_and_canary() {
    let (pw, canary) = parse_secrets("  the phrase  \nCANARY=zzz\n");
    assert_eq!(pw, "the phrase");
    assert_eq!(canary.as_deref(), Some("zzz"));
    let (pw2, canary2) = parse_secrets("only a password\n");
    assert_eq!(pw2, "only a password");
    assert_eq!(canary2, None);
}

/// A synthetic repo built in a temp dir: the gate in local mode catches a planted tombstone hit and a
/// planted override-env assignment. No python and no secrets file are needed for these detections.
#[test]
fn gate_catches_planted_violations_in_a_temp_repo() {
    let dir = std::env::temp_dir().join(format!("stone0-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .expect("git")
    };
    if Command::new("git").arg("--version").output().is_err() {
        eprintln!("git unavailable; skipping temp-repo test");
        return;
    }
    git(&["init", "-q"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    std::fs::write(dir.join("stone0_tombstones.txt"), "retired-phrase-42\n").unwrap();
    std::fs::write(
        dir.join("leaked.md"),
        "someone wrote retired-phrase-42 here\n",
    )
    .unwrap();
    std::fs::write(dir.join("ci.sh"), "STONE0_OVERRIDE=abc make\n").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "init"]);

    let cfg = GateConfig::scan_only(
        dir.clone(),
        dir.join("nonexistent-secrets"),
        dir.join("stone0_tombstones.txt"),
        Mode::Local,
    );
    let report = gate(&cfg);
    assert_eq!(
        report.verdict,
        Verdict::Fail,
        "expected a hard fail, got {report:?}"
    );
    assert!(
        report.failures.iter().any(|f| f.contains("leaked.md")),
        "tombstone hit not reported: {:?}",
        report.failures
    );
    assert!(
        report.failures.iter().any(|f| f.contains("ci.sh")),
        "override-env hit not reported: {:?}",
        report.failures
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A local laundering scan includes nonignored untracked worktree files. The synthetic secrets file is
/// outside the repository so the test proves the note is detected without scanning the secret source.
#[test]
fn gate_catches_a_live_password_in_an_untracked_worktree_file() {
    if Command::new("git").arg("--version").output().is_err() {
        eprintln!("git unavailable; skipping temp-repo test");
        return;
    }

    let base = std::env::temp_dir().join(format!("stone0-untracked-test-{}", std::process::id()));
    let repo = base.join("repo");
    let secrets = base.join("stone0-override.pass");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&repo).unwrap();

    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(args)
            .output()
            .expect("git")
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    std::fs::write(repo.join("README.md"), "clean base\n").unwrap();
    git(&["add", "README.md"]);
    git(&["commit", "-q", "-m", "clean base"]);

    let password = "synthetic-live-password-48291";
    std::fs::write(&secrets, format!("{password}\n")).unwrap();
    std::fs::write(
        repo.join("scratch-note.txt"),
        format!("temporary note: {password}\n"),
    )
    .unwrap();

    let cfg = GateConfig::scan_only(repo, secrets, base.join("missing-tombstones"), Mode::Local);
    let report = gate(&cfg);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("laundered credential")
                && failure.contains("scratch-note.txt")),
        "untracked live-password hit not reported: {:?}",
        report.failures
    );

    let _ = std::fs::remove_dir_all(&base);
}

/// The native scans (tombstone, override-env, canary, and a password that cannot occur) must be CLEAN on
/// the real tracked and nonignored untracked worktree: no false positive. Skips gracefully if git is
/// unavailable.
#[test]
fn the_real_worktree_is_clean_of_native_detections() {
    let root = match repo_root() {
        Some(r) => r,
        None => {
            eprintln!("git toplevel unavailable; skipping real-tree test");
            return;
        }
    };
    let tombstones_path = root.join(TOMBSTONE_REL);
    // Point at a nonexistent secrets file so the live-password and canary scans skip cleanly; this
    // test asserts only that the native tombstone and override-env scans find nothing real.
    let cfg = GateConfig::scan_only(
        root.clone(),
        root.join("crates/stone0/tests/__no_such_secret__"),
        tombstones_path,
        Mode::Local,
    );
    let report = gate(&cfg);
    // The provenance scan may or may not run (python may be absent). Filter to the native detections.
    let native: Vec<&String> = report
        .failures
        .iter()
        .filter(|failure| {
            failure.starts_with("laundered retired override phrase")
                || failure.starts_with("override-env baked into a committed file")
                || failure.starts_with("laundered credential")
                || failure.starts_with("canary tripped")
        })
        .collect();
    assert!(
        native.is_empty(),
        "native false positives on the real tree: {native:?}"
    );
}

fn repo_root() -> Option<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s))
    }
}
