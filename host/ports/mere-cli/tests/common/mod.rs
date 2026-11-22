// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! One invited reader's whole flow over a carrier: the granted query answers,
//! another is refused, a tampered invitation is refused, and a revocation
//! stops it (graphshell:mere-cli Q4, D17).

use std::fs;
use std::path::Path;
use std::process::{Child, Command, Output};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const MERE: &str = env!("CARGO_BIN_EXE_mere");
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
const ROWS: &[u8] =
    b"[[\"Ada\",\"Library catalog\"],[\"Ada\",\"Publisher\"],[\"Grace\",\"Publisher\"]]\n";

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(MERE)
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

/// `serve_extra` and `query_extra` carry what the carrier needs (`--listen`,
/// `--sam`).
pub fn invited_reader(via: &str, serve_extra: &[&str], query_extra: &[&str]) {
    let dir = tempfile::tempdir().unwrap();
    let file = |name| dir.path().join(name);
    fs::write(file("owner.seed"), "01".repeat(32)).unwrap();
    fs::write(file("reader.seed"), "02".repeat(32)).unwrap();
    fs::copy(
        Path::new(FIXTURES).join("paper-authors.rq"),
        file("allowed.rq"),
    )
    .unwrap();
    fs::write(file("other.rq"), "SELECT * WHERE { ?s ?p ?o }").unwrap();
    let contact = run(
        dir.path(),
        &["--identity-seed", "reader.seed", "contact", "--out", "reader.contact"],
    );
    assert!(contact.status.success(), "{}", String::from_utf8_lossy(&contact.stderr));

    let mut server = Server(
        Command::new(MERE)
            .current_dir(dir.path())
            .args(["--identity-seed", "owner.seed", "serve", "--graph"])
            .arg(Path::new(FIXTURES).join("paper-authors.trig"))
            .args([
                "--via", via, "--allow-query", "allowed.rq", "--contact", "reader.contact",
                "--invite", "paper.invite",
            ])
            .args(serve_extra)
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(60);
    while !file("paper.invite").exists() {
        assert!(server.0.try_wait().unwrap().is_none(), "the owner exited early");
        assert!(Instant::now() < deadline, "the owner failed to advertise");
        thread::sleep(Duration::from_millis(50));
    }
    let query = |invite: &str, query: &str| {
        let mut args = vec!["--identity-seed", "reader.seed", "query", "--invite", invite, "--query", query];
        args.extend_from_slice(query_extra);
        run(dir.path(), &args)
    };
    let expect_rows = || {
        let result = query("paper.invite", "allowed.rq");
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
        assert_eq!(result.stdout, ROWS);
    };
    expect_rows();
    let other = query("paper.invite", "other.rq");
    assert!(!other.status.success() && other.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&other.stderr).contains("not enabled"),
        "{}",
        String::from_utf8_lossy(&other.stderr)
    );

    let mut edited: Value = serde_json::from_slice(&fs::read(file("paper.invite")).unwrap()).unwrap();
    let nonce = &mut edited["invitation"]["grant"]["certificate"]["nonce"][0];
    *nonce = json!(nonce.as_u64().unwrap() ^ 1);
    fs::write(file("edited.invite"), serde_json::to_vec(&edited).unwrap()).unwrap();
    let invalid = query("edited.invite", "allowed.rq");
    assert!(!invalid.status.success() && invalid.stdout.is_empty());
    eprintln!("tampered: {}", String::from_utf8_lossy(&invalid.stderr));
    expect_rows();

    let revoke = run(
        dir.path(),
        &["--identity-seed", "owner.seed", "revoke", "--invite", "paper.invite"],
    );
    assert!(revoke.status.success(), "{}", String::from_utf8_lossy(&revoke.stderr));
    thread::sleep(Duration::from_secs(2));
    let revoked = query("paper.invite", "allowed.rq");
    let reason = String::from_utf8_lossy(&revoked.stderr);
    assert!(!revoked.status.success() && revoked.stdout.is_empty());
    assert!(reason.contains("evoked"), "refused for its revocation: {reason}");
}
