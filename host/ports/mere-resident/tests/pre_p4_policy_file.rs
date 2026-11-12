// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A Notochord policy file written by pandect before the store moved to
//! Sporran (engine split D134) is found where the resident's settings
//! directory puts it, loads unchanged, and saves back byte for byte. The
//! path on disk does not change.

use std::fs;
use std::path::Path;

use mere_resident::persona_settings_store::persona_settings_dir;
use notochord::{
    NetworkId, OwnerNetworkPolicy, OwnerPolicyEdit, OwnerPolicySet, ServiceAccess, ServiceRule,
};
use pandect::PersonaId;
use sporran::{NOTOCHORD_POLICY_FILENAME, load_notochord_policy, save_notochord_policy};

/// What `pandect::save_notochord_policy` was given when the fixture was written.
fn written_before_the_move() -> OwnerPolicySet {
    let mut network = OwnerNetworkPolicy::closed(NetworkId([7; 32]));
    network.apply(OwnerPolicyEdit::Service {
        path: "/services/murm".to_string(),
        rule: ServiceRule::new(
            ServiceAccess::MemberOnly,
            "mere.network",
            ["connect"],
            true,
            Some(3),
        ),
    });
    network.apply(OwnerPolicyEdit::Transit(true));
    let mut policies = OwnerPolicySet::new();
    policies.upsert(network);
    policies
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create directory");
    for entry in fs::read_dir(from).expect("read fixture directory") {
        let entry = entry.expect("fixture entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy fixture file");
        }
    }
}

#[test]
fn a_policy_file_written_before_the_move_loads_through_the_resident_path() {
    let root = tempfile::tempdir().expect("scratch root");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pre_p4_policy_root");
    copy_tree(&fixture, root.path());

    let dir = persona_settings_dir(root.path(), PersonaId::default_persona());
    let file = dir.join(NOTOCHORD_POLICY_FILENAME);
    assert!(file.is_file(), "the settings directory names the old path");
    let shown = file.to_string_lossy().replace('\\', "/");
    assert!(
        shown.ends_with("personas/00000000-0000-0000-0000-000000000001/settings/notochord.json"),
        "the path on disk is unchanged: {shown}"
    );

    let loaded = load_notochord_policy(&dir)
        .expect("the old file loads")
        .expect("the old file is found");
    assert_eq!(loaded, written_before_the_move());

    let before = fs::read(&file).expect("read fixture bytes");
    save_notochord_policy(&dir, &loaded).expect("save back");
    assert_eq!(
        fs::read(&file).expect("read saved bytes"),
        before,
        "saving writes the same bytes"
    );
}
