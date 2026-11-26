// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Which mod kinds each target loads today, and why not (mod projects MP8, D11, D46, D47).
//!
//! One table ([`TABLE`]) is the data; [`admit`] reads it. The crate links no Wasm engine and
//! no Lua VM, so the browser build can call it. The kinds line up with the loader's `ModType`
//! (`Native`, `Wasm`, `Lua`) and a project's `ModKind` (`Document`, `Parser`, `Viewer`) without
//! depending on either crate. The plan's Findings table is [`markdown`], pasted and checked.

/// What a mod is, by how it is built and loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A Lua command pack (`ModType::Lua`).
    LuaPack,
    /// A Wasm component exporting `document-core` (`ModKind::Document`).
    DocumentScript,
    /// A Wasm component exporting `parser-guest` (`ModKind::Parser`).
    Parser,
    /// A Wasm component exporting `viewer-guest` (`ModKind::Viewer`).
    Viewer,
    /// A precompiled `.cwasm`, loaded only when the host built it (D4).
    Cwasm,
    /// A function compiled into the binary (`ModType::Native`).
    Native,
}

/// Where the host runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    /// The desktop binary.
    Desktop,
    /// The browser build (`wasm32-unknown-unknown`).
    Browser,
}

/// Whether a kind loads on a target today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    /// Loads today; the note says what that means here, or is empty.
    Allowed(&'static str),
    /// Not today; the text names the path that would open it. A refusal until it ships.
    Planned(&'static str),
    /// Does not load; the text says why.
    Refused(&'static str),
}

impl Admission {
    /// Whether the kind loads today. A planned row does not.
    pub fn loads(self) -> bool {
        matches!(self, Self::Allowed(_))
    }

    /// The reason or note, empty when an allowed row has none.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Allowed(s) | Self::Planned(s) | Self::Refused(s) => s,
        }
    }

    /// One word for a table cell.
    pub fn word(self) -> &'static str {
        match self {
            Self::Allowed(_) => "allowed",
            Self::Planned(_) => "planned",
            Self::Refused(_) => "refused",
        }
    }
}

impl Kind {
    /// Every kind, in table order.
    pub const ALL: [Kind; 6] = [
        Kind::LuaPack,
        Kind::DocumentScript,
        Kind::Parser,
        Kind::Viewer,
        Kind::Cwasm,
        Kind::Native,
    ];

    /// The name a table row carries.
    pub fn name(self) -> &'static str {
        match self {
            Self::LuaPack => "Lua pack",
            Self::DocumentScript => "document-script",
            Self::Parser => "parser",
            Self::Viewer => "viewer",
            Self::Cwasm => ".cwasm",
            Self::Native => "native",
        }
    }
}

impl Target {
    /// Both targets, in table order.
    pub const ALL: [Target; 2] = [Target::Desktop, Target::Browser];

    /// The name a table column carries.
    pub fn name(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Browser => "browser",
        }
    }

    /// The target this crate was compiled for.
    pub fn current() -> Self {
        if cfg!(target_arch = "wasm32") {
            Self::Browser
        } else {
            Self::Desktop
        }
    }
}

const PULLEY: &str = "no Wasm engine in the browser build yet; Wasmtime on Pulley runs this \
                      guest in a page (guest-runtime P7) but is not built in";

/// The table: one row per kind, desktop then browser.
pub const TABLE: [(Kind, [Admission; 2]); 6] = [
    (
        Kind::LuaPack,
        [
            Admission::Allowed(""),
            // script-didge passes `cargo check --target wasm32-unknown-unknown` (Findings 17).
            // That is a statement about the VM; the browser build wires no pack loader yet.
            Admission::Allowed("script-didge builds for wasm32"),
        ],
    ),
    (
        Kind::DocumentScript,
        [Admission::Allowed(""), Admission::Planned(PULLEY)],
    ),
    (
        Kind::Parser,
        [Admission::Allowed(""), Admission::Planned(PULLEY)],
    ),
    (
        Kind::Viewer,
        [Admission::Allowed(""), Admission::Planned(PULLEY)],
    ),
    (
        Kind::Cwasm,
        [
            Admission::Allowed("only when the host built it and its hash matches (D4)"),
            Admission::Refused("machine code built for the host's CPU"),
        ],
    ),
    (
        Kind::Native,
        [
            Admission::Allowed("compiled into that binary only"),
            Admission::Refused(
                "a native mod is a function compiled into the binary; the browser build loads none at runtime",
            ),
        ],
    ),
];

/// Whether `kind` loads on `target` today, and why not.
pub fn admit(kind: Kind, target: Target) -> Admission {
    let col = match target {
        Target::Desktop => 0,
        Target::Browser => 1,
    };
    TABLE
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or(Admission::Refused("unknown kind"), |(_, row)| row[col])
}

/// The message a host shows when `kind` does not load on `target`; `None` when it loads.
pub fn refusal(kind: Kind, target: Target) -> Option<String> {
    let a = admit(kind, target);
    (!a.loads()).then(|| {
        let lead = match a {
            Admission::Planned(_) => "not yet loadable",
            _ => "not loadable",
        };
        format!(
            "{} mods are {lead} in the {} build: {}",
            kind.name(),
            target.name(),
            a.reason()
        )
    })
}

/// The table as Markdown, the form Findings 17 carries.
pub fn markdown() -> String {
    let mut out = String::from("| kind | desktop | browser |\n|---|---|---|\n");
    for (kind, cells) in &TABLE {
        out.push_str(&format!("| {} |", kind.name()));
        for c in cells {
            match c.reason() {
                "" => out.push_str(&format!(" {} |", c.word())),
                r => out.push_str(&format!(" {}: {r} |", c.word())),
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_one_row() {
        for kind in Kind::ALL {
            assert_eq!(
                TABLE.iter().filter(|(k, _)| *k == kind).count(),
                1,
                "{kind:?}"
            );
        }
    }

    #[test]
    fn desktop_loads_every_kind() {
        for kind in Kind::ALL {
            assert!(admit(kind, Target::Desktop).loads(), "{kind:?}");
            assert!(refusal(kind, Target::Desktop).is_none());
        }
    }

    #[test]
    fn browser_refuses_what_it_cannot_load_with_a_reason() {
        for kind in [
            Kind::DocumentScript,
            Kind::Parser,
            Kind::Viewer,
            Kind::Cwasm,
            Kind::Native,
        ] {
            let a = admit(kind, Target::Browser);
            assert!(!a.loads(), "{kind:?}");
            assert!(!a.reason().is_empty(), "{kind:?}");
            assert!(refusal(kind, Target::Browser).is_some());
        }
        assert!(admit(Kind::LuaPack, Target::Browser).loads());
        assert!(refusal(Kind::LuaPack, Target::Browser).is_none());
    }

    #[test]
    fn only_wasm_components_are_planned() {
        for kind in Kind::ALL {
            let planned = matches!(admit(kind, Target::Browser), Admission::Planned(_));
            let component = matches!(kind, Kind::DocumentScript | Kind::Parser | Kind::Viewer);
            assert_eq!(planned, component, "{kind:?}");
        }
    }

    #[test]
    fn markdown_has_a_line_per_kind() {
        assert_eq!(markdown().lines().count(), 2 + Kind::ALL.len());
    }
}
