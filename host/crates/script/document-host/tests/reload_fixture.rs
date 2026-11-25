// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Mod projects MP3: reload as detach and attach to the same document, fixtures,
//! and traps named in the record (D3, D6). The standard guest edits the document;
//! the bomb guest (imports nothing) answers every ordinary turn with an empty batch citing revision 0,
//! so swapping one for the other is "the same project before and after an edit".

use std::path::{Path, PathBuf};

use document_host::mod_loader::{
    AdmissionLevel, InstallRecord, ModCapability, ModManifest, ModType,
};
use document_host::runtime::DocumentScriptRuntime;
use document_host::{
    CapPermission, DocumentScript, Fixture, FixtureOutcome, Grant, Quota, ReloadError,
    TurnOutcome, run_fixture,
};
use genet_scripted_dom::ScriptedDom;
use layout_dom_api::{LayoutDom, LayoutDomMut, LocalName, Namespace, QualName};

fn fixture(var: &str, default: &str) -> PathBuf {
    let p = std::env::var(var).unwrap_or_else(|_| default.to_string());
    let path = PathBuf::from(&p);
    assert!(path.exists(), "fixture guest missing at {p}");
    path
}

fn guest() -> PathBuf {
    fixture(
        "DOC_HOST_GUEST_WASM",
        "guest/target/wasm32-wasip2/release/document_core_guest.wasm",
    )
}

fn bomb() -> PathBuf {
    fixture(
        "DOC_HOST_BOMB_WASM",
        "guest-bomb/target/wasm32-wasip2/release/document_core_guest_bomb.wasm",
    )
}

fn qual(local: &str) -> QualName {
    QualName::new(None, Namespace::from(""), LocalName::from(local))
}

/// `<body><p>Hello</p></body>`.
fn page() -> ScriptedDom {
    let mut dom = ScriptedDom::new();
    let root = dom.document();
    let body = dom.create_element(qual("body"));
    dom.append_child(root, body);
    let p = dom.create_element(qual("p"));
    dom.append_child(body, p);
    let t = dom.create_text("Hello");
    dom.append_child(p, t);
    dom
}

/// The text under `<body>`'s first paragraph.
fn first_text(dom: &ScriptedDom) -> String {
    let body = dom.dom_children(dom.document()).next().expect("body");
    let p = dom.dom_children(body).next().expect("p");
    let t = dom.dom_children(p).next().expect("text");
    dom.text(t).expect("text").to_string()
}

fn attach(path: &Path) -> DocumentScript {
    DocumentScript::attach(path, page(), &Grant::allow_all(), Quota::default(), None, vec![])
        .expect("attach")
}

fn reload(script: &mut DocumentScript, path: &Path, grant: &Grant) -> Result<(), ReloadError> {
    script.reload(path, grant, Quota::default(), None, vec![], None)
}

fn no_net() -> Grant {
    Grant {
        net: CapPermission::Deny,
        ..Grant::allow_all()
    }
}

#[test]
fn a_reload_changes_behaviour_on_the_same_document() {
    let mut script = attach(&guest());
    assert_eq!(
        script.deliver_event("set", "one").expect("turn"),
        TurnOutcome::Applied(1)
    );
    assert!(script.guest_logs().iter().any(|l| l.contains("turn 'set'")));

    // The bomb guest never edits: the document stays, the revision stays, and the
    // old instance's log is gone with the instance.
    reload(&mut script, &bomb(), &Grant::allow_all()).expect("reload to the bomb guest");
    assert_eq!(first_text(script.dom()), "one");
    assert_eq!(script.revision(), 1);
    assert!(script.guest_logs().is_empty());
    assert_eq!(
        script.deliver_event("set", "two").expect("turn"),
        TurnOutcome::Conflict(1),
        "the bomb guest cites revision 0: new behaviour, same document at revision 1"
    );
    assert_eq!(first_text(script.dom()), "one");

    // And back: the guest edits the same document, the revision carrying on from 1.
    reload(&mut script, &guest(), &Grant::allow_all()).expect("reload to the guest");
    assert_eq!(
        script.deliver_event("set", "three").expect("turn"),
        TurnOutcome::Applied(2)
    );
    assert_eq!(first_text(script.dom()), "three");
}

#[test]
fn a_failed_reload_keeps_the_old_instance_attached() {
    let mut script = attach(&guest());
    script.deliver_event("set", "kept").expect("turn");

    // Check fails: the guest imports `net`, which this grant does not link.
    let err = reload(&mut script, &guest(), &no_net()).expect_err("check refuses");
    let ReloadError::Check(report) = &err else {
        panic!("wanted a check failure, got {err:?}");
    };
    assert_eq!(report.ungranted_imports, vec!["mere:script/net".to_string()]);

    // Not a component at all.
    let dir = tempfile::tempdir().expect("tempdir");
    let junk = dir.path().join("junk.wasm");
    std::fs::write(&junk, b"not a component").expect("write");
    let err = reload(&mut script, &junk, &Grant::allow_all()).expect_err("junk refused");
    assert!(matches!(err, ReloadError::Refused(_)), "{err:?}");

    // A pinned component that changed fails at instantiation, not at check.
    let rt = DocumentScriptRuntime::new(vec![ModCapability::Network]);
    let manifest = ModManifest::new("mod:pinned", "pinned", ModType::Wasm, vec![], vec![], vec![]);
    let mut record = rt
        .admit(&manifest, "1", &bomb(), AdmissionLevel::Conformant, false)
        .expect("admit");
    let err = script
        .reload(&guest(), &Grant::allow_all(), Quota::default(), None, vec![], Some(&mut record))
        .expect_err("hash differs");
    assert!(matches!(err, ReloadError::Instantiate(_)), "{err:?}");
    assert!(err.words().contains("changed since admission"), "{err}");
    assert!(record.last_trap.is_some());

    // The old instance still runs against the untouched document.
    assert_eq!(first_text(script.dom()), "kept");
    assert_eq!(
        script.deliver_event("set", "still here").expect("turn"),
        TurnOutcome::Applied(2)
    );
    assert_eq!(first_text(script.dom()), "still here");
}

#[test]
fn a_failed_check_on_reload_is_kept_on_the_record() {
    let rt = DocumentScriptRuntime::new(vec![ModCapability::Network]);
    let manifest = ModManifest::new(
        "mod:reloaded",
        "reloaded",
        ModType::Wasm,
        vec![],
        vec![],
        vec![ModCapability::Network],
    );
    let mut record = rt
        .admit(&manifest, "1", &guest(), AdmissionLevel::DevMode, false)
        .expect("admit");
    let mut script = attach(&guest());
    script
        .reload(&guest(), &no_net(), Quota::default(), None, vec![], Some(&mut record))
        .expect_err("check refuses");
    let note = record.last_check.expect("check kept").note;
    assert!(note.contains("mere:script/net"), "{note}");
}

const SET_FIRST: &str = include_str!("fixtures/wasm_mod/set_first.toml");

fn run(fixture: &Fixture, component: &Path, record: Option<&mut InstallRecord>) -> FixtureOutcome {
    run_fixture(
        fixture,
        component,
        &Grant::allow_all(),
        Quota {
            epoch_deadline_ticks: 10,
            ..Quota::default()
        },
        None,
        vec![],
        record,
    )
    .expect("the fixture runs")
}

#[test]
fn a_passing_fixture_passes() {
    let fixture = Fixture::from_toml(SET_FIRST).expect("parses");
    let outcome = run(&fixture, &guest(), None);
    assert!(outcome.passed(), "{}", outcome.words());
    let FixtureOutcome::Pass(run) = outcome else {
        unreachable!()
    };
    assert!(run.guest_logs.iter().any(|l| l.contains("turn 'append'")));
}

#[test]
fn a_failing_fixture_shows_the_diff() {
    // The bomb guest leaves the document alone, so both expectations miss.
    let fixture = Fixture::from_toml(SET_FIRST).expect("parses");
    let outcome = run(&fixture, &bomb(), None);
    let FixtureOutcome::Mismatch {
        dom_diff, log_diff, ..
    } = &outcome
    else {
        panic!("wanted a mismatch");
    };
    let dom_diff = dom_diff.as_deref().expect("dom differs");
    assert!(dom_diff.contains("-     #text \"Changed\""), "{dom_diff}");
    assert!(dom_diff.contains("+     #text \"Hello\""), "{dom_diff}");
    let log_diff = log_diff.as_deref().expect("log differs");
    assert!(log_diff.contains("- 1 set: applied, rev 1"), "{log_diff}");
    assert!(log_diff.contains("+ 1 set: applied, rev 0"), "{log_diff}");
    assert!(outcome.words().contains("dom differs"));
}

#[test]
fn a_trap_is_named_in_the_log_and_on_the_record() {
    let text = r#"
name = "spin"
[document]
tree = "body\n  p\n    #text \"Hello\"\n"
[[turn]]
kind = "noop"
[[turn]]
kind = "loop"
[[turn]]
kind = "never-reached"
[expect]
dom = "body\n  p\n    #text \"Hello\"\n"
log = ["1 noop: applied, rev 0", "2 loop: trapped"]
"#;
    let fixture = Fixture::from_toml(text).expect("parses");
    let rt = DocumentScriptRuntime::permissive();
    let manifest = ModManifest::new("mod:spin", "spin", ModType::Wasm, vec![], vec![], vec![]);
    let mut record = rt
        .admit(&manifest, "1", &bomb(), AdmissionLevel::Conformant, false)
        .expect("admit");
    let outcome = run(&fixture, &bomb(), Some(&mut record));
    assert!(outcome.passed(), "{}", outcome.words());
    let trap = record.last_trap.expect("trap kept").note;
    assert!(trap.starts_with("turn 2 'loop':"), "{trap}");
}

#[test]
fn an_attach_failure_names_the_import_on_the_record() {
    let fixture = Fixture::from_toml(SET_FIRST).expect("parses");
    let rt = DocumentScriptRuntime::permissive();
    let manifest = ModManifest::new("mod:nonet", "nonet", ModType::Wasm, vec![], vec![], vec![]);
    let mut record = rt
        .admit(&manifest, "1", &guest(), AdmissionLevel::Conformant, false)
        .expect("admit");
    let err = run_fixture(
        &fixture,
        &guest(),
        &no_net(),
        Quota::default(),
        None,
        vec![],
        Some(&mut record),
    )
    .expect_err("attach fails");
    assert!(err.contains("mere:script/net"), "{err}");
    let trap = record.last_trap.expect("trap kept").note;
    assert!(trap.contains("mere:script/net"), "{trap}");
}

#[test]
fn a_malformed_fixture_is_refused() {
    let bad = SET_FIRST.replace("#text \"Hello\"", "#text Hello");
    assert!(Fixture::from_toml(&bad).is_err());
    let bad = SET_FIRST.replace("    #text \"Hello\"", "      #text \"Hello\"");
    assert!(Fixture::from_toml(&bad).unwrap_err().contains("indentation"));
}

#[test]
fn a_fixture_reads_from_a_file() {
    let path = Path::new("tests/fixtures/wasm_mod/set_first.toml");
    let fixture = Fixture::from_file(path).expect("reads");
    assert_eq!(fixture.turn.len(), 2);
}
