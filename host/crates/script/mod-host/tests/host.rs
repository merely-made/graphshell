// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The `parser-guest` and `viewer-guest` worlds end to end over real components built in
//! Mere: the claim, the host fixing the document's address, provenance and trust, the caps
//! on payloads, a trap or a spin ending as a value, pinning, and the TOML fixture forms.

use std::path::{Path, PathBuf};

use document_host::mod_loader::{
    AdmissionLevel, InstallRecord, ModManifest, ModType, admit_wasm,
};
use document_host::{Grant, check};
use inker::DocumentTrustState;
use mod_host::fixture::{
    FixtureOutcome, ParserFixture, ViewerFixture, outline, run_parser_fixture, run_viewer_fixture,
};
use mod_host::test_support::{parser_guest_path, parser_guest_wasm, viewer_guest_path, viewer_guest_wasm};
use mod_host::{
    Limits, ModKind, ParseContext, ParseFailure, ParserHost, Severity, ViewFailure, ViewerHost,
    guarded_engine, watchdog,
};

const NOTES: &str = "application/x-fixture-notes";

fn limits(ticks: u64) -> Limits {
    Limits { epoch_deadline_ticks: ticks, ..Limits::default() }
}

fn ctx() -> ParseContext<'static> {
    ParseContext {
        address: "http://127.0.0.1/page.notes",
        engine_id: "mod:fixture",
        trust: DocumentTrustState::Insecure,
    }
}

fn parser(ticks: u64) -> (ParserHost, mod_host::wasmtime::Engine, mod_host::Watchdog) {
    let engine = guarded_engine().unwrap();
    let dog = watchdog(&engine);
    let host = ParserHost::attach_bytes(
        &engine,
        &parser_guest_wasm().unwrap(),
        None,
        vec!["mere:script/log".into()],
        limits(ticks),
    )
    .unwrap();
    (host, engine, dog)
}

fn viewer(ticks: u64) -> (ViewerHost, mod_host::Watchdog) {
    let engine = guarded_engine().unwrap();
    let dog = watchdog(&engine);
    let host = ViewerHost::attach_bytes(&engine, &viewer_guest_wasm().unwrap(), None, vec![], limits(ticks)).unwrap();
    (host, dog)
}

fn fixture_dir(kind: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(kind)
}

#[test]
fn describe_claims_normalized_media_types() {
    let (mut host, _engine, _dog) = parser(100);
    let types = host.describe().unwrap();
    assert!(types.contains(&NOTES.to_string()) && types.len() == 8, "{types:?}");
}

#[test]
fn a_parse_returns_blocks_and_diagnostics_with_the_hosts_own_provenance() {
    let (mut host, _engine, _dog) = parser(100);
    let bytes = b"# Title\nbody\n\ttabbed\n";
    let parsed = host.parse(bytes, "Application/X-Fixture-Notes; charset=utf-8", &ctx()).unwrap();
    assert_eq!(parsed.document.title.as_deref(), Some("Title"));
    assert_eq!(outline(&parsed.document), ["heading 1: Title", "paragraph: body", "paragraph: tabbed"]);
    assert_eq!(parsed.document.content_type, NOTES);
    assert_eq!(parsed.diagnostics.len(), 1);
    assert_eq!(parsed.diagnostics[0].severity, Severity::Warning);
    assert_eq!(parsed.diagnostics[0].span, Some((13, 21)));
    assert_eq!(parsed.document.address, "http://127.0.0.1/page.notes");
    assert_eq!(parsed.document.provenance.source_kind.as_deref(), Some("mod:fixture"));
    assert_eq!(parsed.document.trust, DocumentTrustState::Insecure);
}

#[test]
fn the_host_overwrites_what_a_lying_guest_claims() {
    let (mut host, _engine, _dog) = parser(100);
    let parsed = host.parse(b"# T\n", "application/x-fixture-liar", &ctx()).unwrap();
    let doc = parsed.document;
    assert_eq!(doc.address, "http://127.0.0.1/page.notes");
    assert_eq!(doc.trust, DocumentTrustState::Insecure);
    assert_eq!(doc.provenance.source_kind.as_deref(), Some("mod:fixture"));
    assert_eq!(doc.provenance.canonical_uri.as_deref(), Some("http://127.0.0.1/page.notes"));
    assert_eq!(doc.provenance.source_label, None, "the guest's label is dropped");
}

#[test]
fn refusals_and_bad_payloads_are_values_and_keep_the_instance() {
    let (mut host, _engine, _dog) = parser(100);
    let c = ctx();
    assert_eq!(
        host.parse(b"x", "application/x-fixture-refuse", &c).unwrap_err(),
        ParseFailure::Refused("fixture refuses this input".into())
    );
    assert_eq!(
        host.parse(b"x", "application/x-fixture-badversion", &c).unwrap_err(),
        ParseFailure::BadVersion(9)
    );
    let huge = host.parse(b"x", "application/x-fixture-huge", &c).unwrap_err();
    assert!(matches!(huge, ParseFailure::TooLarge(_)), "{huge:?}");
    assert!(!huge.drops_instance());
    assert!(matches!(
        host.parse(b"x", "application/x-fixture-nope", &c).unwrap_err(),
        ParseFailure::Refused(_)
    ));
    // The same instance still parses.
    assert!(host.parse(b"# ok\n", NOTES, &c).is_ok());
}

#[test]
fn a_trap_and_a_spin_end_as_trapped_without_taking_the_host() {
    let (mut host, _engine, _dog) = parser(100);
    let trap = host.parse(b"x", "application/x-fixture-trap", &ctx()).unwrap_err();
    assert!(matches!(trap, ParseFailure::Trapped(_)) && trap.drops_instance(), "{trap:?}");
    let ParseFailure::Trapped(cause) = &trap else { unreachable!() };
    assert!(cause.contains("unreachable"), "the cause, not the backtrace header: {cause}");
    assert!(!cause.contains("backtrace"), "{cause}");

    let (mut host, _engine, _dog) = parser(5);
    let started = std::time::Instant::now();
    let spin = host.parse(b"x", "application/x-fixture-spin", &ctx()).unwrap_err();
    assert!(matches!(spin, ParseFailure::Trapped(_)), "{spin:?}");
    let ParseFailure::Trapped(cause) = &spin else { unreachable!() };
    assert!(cause.contains("interrupt"), "a spin names the interrupt: {cause}");
    assert!(started.elapsed() < std::time::Duration::from_secs(5), "the watchdog ended it");
}

#[test]
fn diagnostics_are_capped_and_the_last_says_so() {
    let (mut host, _engine, _dog) = parser(200);
    let parsed = host.parse(b"x", "application/x-fixture-chatty", &ctx()).unwrap();
    assert_eq!(parsed.diagnostics.len(), mod_host::MAX_DIAGNOSTICS + 1);
    assert!(parsed.diagnostics.last().unwrap().message.contains("300 diagnostics"));
}

#[test]
fn an_oversize_input_and_component_are_refused() {
    let (mut host, _engine, _dog) = parser(100);
    let big = vec![b'a'; mod_host::MAX_DOCUMENT_BYTES + 1];
    assert!(matches!(host.parse(&big, NOTES, &ctx()).unwrap_err(), ParseFailure::TooLarge(_)));
    let engine = guarded_engine().unwrap();
    let too_big = vec![0u8; mod_host::MAX_COMPONENT_BYTES + 1];
    let error = ParserHost::attach_bytes(&engine, &too_big, None, vec![], Limits::default()).err().unwrap();
    assert!(error.to_string().contains("exceeds"));
}

fn record_for(path: &Path) -> InstallRecord {
    let manifest = ModManifest::new("fixture", "Fixture", ModType::Wasm, vec![], vec![], vec![]);
    admit_wasm(&manifest, "0.0.1", path, AdmissionLevel::Conformant, &[], None).unwrap()
}

#[test]
fn attaching_against_a_record_refuses_a_component_that_is_not_the_pinned_one() {
    let engine = guarded_engine().unwrap();
    let record = record_for(&parser_guest_path());
    let own = parser_guest_wasm().unwrap();
    assert!(ParserHost::attach_bytes(&engine, &own, Some(&record), vec![], Limits::default()).is_ok());
    let other = viewer_guest_wasm().unwrap();
    let error = ParserHost::attach_bytes(&engine, &other, Some(&record), vec![], Limits::default()).err().unwrap();
    assert!(error.to_string().contains("changed") || error.to_string().contains("pinned"), "{error}");
    let vrecord = record_for(&viewer_guest_path());
    assert!(ViewerHost::attach_bytes(&engine, &own, Some(&vrecord), vec![], Limits::default()).is_err());
}

#[test]
fn check_compares_each_component_with_its_own_world() {
    let grant = Grant::allow_all();
    let report = check(&parser_guest_path(), &grant, &ModKind::Parser.world()).unwrap();
    assert!(report.ok(), "{}", report.words());
    let wrong = check(&parser_guest_path(), &grant, &ModKind::Viewer.world()).unwrap();
    assert_eq!(wrong.missing_exports, ["mere:script/viewer"]);
    let report = check(&viewer_guest_path(), &grant, &ModKind::Viewer.world()).unwrap();
    assert!(report.ok(), "{}", report.words());
}

const GRAPH: &[u8] = br#"{"snapshot":{"nodes":[{"label":"A"},{"label":"B"}]},"facets":{}}"#;

#[test]
fn a_viewer_returns_a_decoded_scene_and_sheet() {
    let (mut host, _dog) = viewer(100);
    assert_eq!(host.title(), "Fixture nodes");
    let view = host.view(GRAPH, 320.0).unwrap();
    assert_eq!(view.region_labels(), ["Node A", "Node B"]);
    assert_eq!(view.width, 320.0);
    assert_eq!(view.sheet.width, view.width);
    assert_eq!(view.description, "2 nodes");
}

#[test]
fn a_viewer_refusal_a_trap_and_a_spin_are_values() {
    let (mut host, _dog) = viewer(100);
    assert!(matches!(host.view(b"not json", 100.0).unwrap_err(), ViewFailure::Refused(_)));
    let trap = host.view(br#"{"snapshot":{"nodes":[{"label":"TRAP"}]}}"#, 100.0).unwrap_err();
    assert!(matches!(trap, ViewFailure::Trapped(_)) && trap.drops_instance(), "{trap:?}");
    let ViewFailure::Trapped(cause) = &trap else { unreachable!() };
    assert!(cause.contains("unreachable"), "the cause, not the backtrace header: {cause}");
    assert!(!cause.contains("backtrace"), "{cause}");
    let (mut host, _dog) = viewer(5);
    let spin = host.view(br#"{"snapshot":{"nodes":[{"label":"SPIN"}]}}"#, 100.0).unwrap_err();
    assert!(matches!(spin, ViewFailure::Trapped(_)), "{spin:?}");
}

#[test]
fn the_viewers_items_are_centred_on_their_translate_and_a_title_trap_traps() {
    let (mut host, _dog) = viewer(100);
    let view = host
        .view(br#"{"snapshot":{"nodes":[{"title":"A"},{"title":"B"}]},"facets":{}}"#, 320.0)
        .unwrap();
    let centres: Vec<(f32, f32)> =
        view.scene.items.iter().map(|i| (i.transform.translate.x, i.transform.translate.y)).collect();
    assert_eq!(centres, [(20.0, 20.0), (20.0, 50.0)], "the 20-square at (10, 10) is centred at (20, 20)");
    assert!(view.sheet.ops.iter().any(|op| matches!(op, vector_sheet::Op::Text { text, .. } if text == "B")));
    let trap = host.view(br#"{"snapshot":{"nodes":[{"title":"TRAP"}]}}"#, 100.0).unwrap_err();
    assert!(matches!(trap, ViewFailure::Trapped(_)), "{trap:?}");
}

#[test]
fn a_viewer_that_reads_another_graph_format_does_not_attach() {
    // The parser component has no viewer export: instantiation itself fails.
    let engine = guarded_engine().unwrap();
    assert!(ViewerHost::attach_bytes(&engine, &parser_guest_wasm().unwrap(), None, vec![], Limits::default()).is_err());
}

fn parser_outcome(name: &str, record: Option<&mut InstallRecord>) -> FixtureOutcome {
    let fixture = ParserFixture::from_file(&fixture_dir("parser").join(name)).unwrap();
    run_parser_fixture(&fixture, &parser_guest_path(), limits(100), record).unwrap()
}

fn viewer_outcome(name: &str) -> FixtureOutcome {
    let fixture = ViewerFixture::from_file(&fixture_dir("viewer").join(name)).unwrap();
    run_viewer_fixture(&fixture, &viewer_guest_path(), limits(100), None).unwrap()
}

#[test]
fn parser_fixtures_pass_and_a_wrong_outline_shows_its_diff() {
    for name in ["notes.toml", "refuse.toml", "trap.toml"] {
        let outcome = parser_outcome(name, None);
        assert!(outcome.passed(), "{name}: {}", outcome.words());
    }
    let wrong = parser_outcome("wrong_outline.toml", None);
    assert!(!wrong.passed());
    let words = wrong.words();
    assert!(words.contains("- heading 2: One") && words.contains("+ heading 1: One"), "{words}");
}

#[test]
fn a_trapping_parser_fixture_is_kept_as_the_records_last_trap() {
    let mut record = record_for(&parser_guest_path());
    assert!(parser_outcome("trap.toml", Some(&mut record)).passed());
    assert!(record.last_trap.as_ref().unwrap().note.contains("trapped"));
    let mut quiet = record_for(&parser_guest_path());
    assert!(parser_outcome("notes.toml", Some(&mut quiet)).passed());
    assert!(quiet.last_trap.is_none());
}

#[test]
fn viewer_fixtures_expect_regions_and_a_size_range() {
    for name in ["nodes.toml", "titles.toml", "trap.toml"] {
        let outcome = viewer_outcome(name);
        assert!(outcome.passed(), "{name}: {}", outcome.words());
    }
    let wrong = viewer_outcome("wrong_regions.toml");
    let words = wrong.words();
    assert!(words.contains("- regions: 2") && words.contains("+ regions: 1"), "{words}");
    assert!(words.contains("height: 40 outside 500..600"), "{words}");
}

#[test]
fn fixture_forms_refuse_bad_input_by_name() {
    let both = "[input]\nmedia_type = \"a/b\"\ntext = \"x\"\nhex = \"00\"\n";
    assert!(ParserFixture::from_toml(both).unwrap_err().contains("either text or hex"));
    let junk = "[input]\nmedia_type = \"nope\"\ntext = \"x\"\n";
    assert!(ParserFixture::from_toml(junk).unwrap_err().contains("not a media type"));
    let hex = "[input]\nmedia_type = \"a/b\"\nhex = \"ff 00\"\n";
    assert_eq!(ParserFixture::from_toml(hex).unwrap().input_bytes().unwrap(), [0xff, 0]);
    assert!(ViewerFixture::from_toml("[graph]\njson = \"[]\"\n").is_err());
}
