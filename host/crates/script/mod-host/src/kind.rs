// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The kinds of mod project and the media types a parser declares.

use document_host::World;

/// What a project's component is: the world it must export (`kind` in `mod.toml`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ModKind {
    /// A document script: `document-core`.
    #[default]
    Document,
    /// A parser: `parser-guest`.
    Parser,
    /// A viewer: `viewer-guest`.
    Viewer,
}

impl ModKind {
    /// The word `mod.toml` carries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Parser => "parser",
            Self::Viewer => "viewer",
        }
    }

    /// # Errors
    ///
    /// The word is not one of the three kinds; the message names them.
    pub fn parse(word: &str) -> Result<Self, String> {
        match word {
            "document" => Ok(Self::Document),
            "parser" => Ok(Self::Parser),
            "viewer" => Ok(Self::Viewer),
            other => Err(format!(
                "unknown kind '{other}'; a project is a document, parser or viewer"
            )),
        }
    }

    /// The world the host `check` compares a component's exports with.
    pub fn world(self) -> World {
        match self {
            Self::Document => World::document_core(),
            Self::Parser => World {
                name: "parser-guest".into(),
                exports: vec!["mere:script/parser".into()],
            },
            Self::Viewer => World {
                name: "viewer-guest".into(),
                exports: vec!["mere:script/viewer".into()],
            },
        }
    }
}

/// A media type reduced to its essence: parameters dropped, lowercase. `None` when it is
/// not `type/subtype` of token characters, so a manifest cannot declare junk.
pub fn normalize_media_type(raw: &str) -> Option<String> {
    let essence = raw.split(';').next()?.trim().to_ascii_lowercase();
    let (kind, sub) = essence.split_once('/')?;
    let token = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$&-^_.+".contains(&b))
    };
    (token(kind) && token(sub)).then_some(essence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_round_trip_and_unknown_names_the_three() {
        for kind in [ModKind::Document, ModKind::Parser, ModKind::Viewer] {
            assert_eq!(ModKind::parse(kind.as_str()), Ok(kind));
        }
        assert!(ModKind::parse("lens").unwrap_err().contains("document, parser or viewer"));
        assert_eq!(ModKind::Parser.world().exports, ["mere:script/parser"]);
    }

    #[test]
    fn media_types_lose_parameters_and_case_and_junk_is_refused() {
        assert_eq!(
            normalize_media_type("Application/X-Notes; charset=utf-8").as_deref(),
            Some("application/x-notes")
        );
        assert_eq!(normalize_media_type("text/plain").as_deref(), Some("text/plain"));
        for bad in ["", "text", "/x", "text/", "te xt/plain", "a/b/c"] {
            assert_eq!(normalize_media_type(bad), None, "{bad}");
        }
    }
}
