// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Recipes as authored TOML (table projection plan, T1). JSON stays the
//! carried form; this is the form a person writes and a pack ships.

use crate::{AuthoredProjectionDefinition, ProjectionDefinition};

macro_rules! toml_io {
    ($recipe:ty) => {
        impl $recipe {
            pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
                toml::to_string(self)
            }

            /// Parse only; call `validate` before trusting the result.
            pub fn from_toml(text: &str) -> Result<Self, toml::de::Error> {
                toml::from_str(text)
            }
        }
    };
}

toml_io!(ProjectionDefinition);
toml_io!(AuthoredProjectionDefinition);

#[cfg(test)]
mod tests {
    use crate::{FieldSpec, ProjectionDefinition};

    const RECIPE: &str = r#"
version = 1
id = "notes-table"
label = "Notes"

[source]
authority = "local-device"
domain = "notes"
resource = "graph:notes"

[reading]
kind = "nodes"
key = "id"

[encoding]
x = { kind = "field", value = "order" }
y = { kind = "constant", value = "0" }
label = { kind = "field", value = "title" }

[arrangement]
kind = "grid"
direction = "coordinates"
spacing = 16

[[fields]]
path = "title"
name = "Title"
description = "What the note is called"

[[fields]]
path = "state"
name = "State"
group = "Status"
viewer = "chip"

[interaction]
selection = "single"
pan = false
zoom = false

[appearance]
realization = "canvas"
title = "Notes"
theme = "slate"

[provenance]
author = "mark"
note = ""
"#;

    #[test]
    fn a_recipe_written_by_hand_reads_and_round_trips() {
        let recipe = ProjectionDefinition::from_toml(RECIPE).expect("reads");
        assert_eq!(
            recipe.fields,
            [
                FieldSpec {
                    description: "What the note is called".into(),
                    ..FieldSpec::new("title", "Title")
                },
                FieldSpec {
                    group: Some("Status".into()),
                    viewer: Some("chip".into()),
                    ..FieldSpec::new("state", "State")
                },
            ]
        );
        // An absent optional value is absent, not an error, both ways.
        assert_eq!(recipe.reading.value, None);
        let again = ProjectionDefinition::from_toml(&recipe.to_toml().expect("writes"));
        assert_eq!(again.expect("reads back"), recipe);
        // The carried form holds the same recipe.
        let json = recipe.to_json_bytes().expect("json");
        let carried: ProjectionDefinition = serde_json::from_slice(&json).expect("json reads");
        assert_eq!(carried, recipe);
    }

    #[test]
    fn a_recipe_without_fields_writes_no_fields_key() {
        let mut recipe = ProjectionDefinition::from_toml(RECIPE).expect("reads");
        recipe.fields.clear();
        assert!(!recipe.to_toml().expect("writes").contains("fields"));
        let json = String::from_utf8(recipe.to_json_bytes().expect("json")).expect("utf8");
        assert!(!json.contains("fields"));
    }
}
