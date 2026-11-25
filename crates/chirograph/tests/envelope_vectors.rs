// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The envelope's interoperability vectors (engine split D210). Any drift in
//! the encoding, a renamed field included, fails here; a deliberate change
//! rewrites them with `CHIROGRAPH_BLESS_VECTORS=1` and bumps the schema in
//! `wire/projection.cddl`.

use std::path::PathBuf;

use cbor_core::Value;
use chirograph::Revision;
use chirograph::envelope::{self, Envelope, EnvelopeKind};

#[path = "envelope_vectors/fixture.rs"]
mod fixture;
use fixture::{diff, engine, keyframe};

fn vectors() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("wire/vectors")
}

/// Diagnostic notation of an envelope, its payload written as `<< ... >>` so
/// the scene reads as text and still parses to the same bytes.
fn diagnostic(bytes: &[u8]) -> String {
    let mut envelope = Value::decode_owned(bytes).unwrap();
    let payload = envelope.remove("payload").unwrap();
    let payload = Value::decode(payload.as_bytes().unwrap()).unwrap();
    envelope.insert("payload", Value::from("\u{1}payload"));
    let text =
        format!("{envelope:#?}").replace("\"\\u0001payload\"", &format!("<<\n{payload:#?}\n>>"));
    // cbor-core's pretty notation ends every entry with a comma, and its
    // parser refuses one before a closing bracket.
    let lines: Vec<&str> = text.lines().collect();
    let mut out = String::new();
    for (at, line) in lines.iter().enumerate() {
        let closes = lines
            .get(at + 1)
            .is_some_and(|next| next.trim_start().starts_with(['}', ']', '>']));
        out.push_str(if closes {
            line.strip_suffix(',').unwrap_or(line)
        } else {
            line
        });
        out.push('\n');
    }
    out
}

fn hex(bytes: &[u8]) -> String {
    let text: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let mut lines: Vec<&str> = Vec::new();
    for at in (0..text.len()).step_by(64) {
        lines.push(&text[at..(at + 64).min(text.len())]);
    }
    lines.join("\n") + "\n"
}

fn unhex(text: &str) -> Vec<u8> {
    let digits: String = text.split_whitespace().collect();
    (0..digits.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&digits[at..at + 2], 16).unwrap())
        .collect()
}

/// The checked-in vector named `name`: its hex is these bytes, its
/// diagnostic notation parses to them, and they open as `value`.
fn check<P>(name: &str, value: &P)
where
    P: envelope::Enveloped + PartialEq + std::fmt::Debug,
{
    let bytes = envelope::seal(value).unwrap();
    let (hex_path, diag_path) = (
        vectors().join(format!("{name}.hex")),
        vectors().join(format!("{name}.diag")),
    );
    if std::env::var_os("CHIROGRAPH_BLESS_VECTORS").is_some() {
        std::fs::create_dir_all(vectors()).unwrap();
        std::fs::write(&hex_path, hex(&bytes)).unwrap();
        std::fs::write(&diag_path, diagnostic(&bytes)).unwrap();
    }
    let pinned = unhex(&std::fs::read_to_string(&hex_path).unwrap());
    if pinned != bytes {
        let at = pinned.iter().zip(&bytes).position(|(a, b)| a != b);
        panic!(
            "{name}: the encoding drifted from wire/vectors/{name}.hex (first difference at \
             byte {at:?}; {} pinned bytes, {} now). Bump the schema and bless deliberately.",
            pinned.len(),
            bytes.len()
        );
    }
    let diag: Value = std::fs::read_to_string(&diag_path)
        .unwrap()
        .parse()
        .expect("the diagnostic vector parses");
    assert_eq!(
        diag.encode(),
        pinned,
        "{name}: the diagnostic notation is these bytes"
    );
    assert_eq!(
        &envelope::open::<P>(&pinned).unwrap(),
        value,
        "{name}: the pinned bytes open"
    );
}

#[test]
fn the_keyframe_vector_holds() {
    check("keyframe", &keyframe());
    let pinned = unhex(&std::fs::read_to_string(vectors().join("keyframe.hex")).unwrap());
    let header = Envelope::read(&pinned).unwrap();
    assert_eq!(header.kind, EnvelopeKind::Keyframe);
    assert_eq!(header.identity.engine, engine(108));
}

#[test]
fn the_diff_vector_holds() {
    check("diff", &diff());
}

#[test]
fn the_vectors_apply_in_sequence() {
    let mut scene = keyframe().scene;
    scene
        .apply_diff(&diff().scene)
        .expect("the diff applies to the keyframe");
    assert_eq!(scene.revision, Revision(15));
    assert_eq!(scene.active_item_count(), 4);
    assert_eq!(scene.engine_revision, Some(engine(109)));
}
