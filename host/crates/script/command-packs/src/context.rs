// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! What a pack command reads (command packs D4): the selection, the focused
//! view and the command's arguments, under a versioned schema that hosts
//! extend through `extensions`.

use std::collections::BTreeMap;

use script_engine_api::ScriptValue;

/// The schema version a script sees as `context.version`.
pub const CONTEXT_VERSION: u32 = 1;

/// The focused view: its kind and, when it has one, its target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Focus {
    pub kind: String,
    pub target: Option<String>,
}

/// A snapshot the script reads and cannot change. Plain data, so a host builds
/// one per run.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandContext {
    pub version: u32,
    pub selection: Vec<String>,
    pub focus: Option<Focus>,
    pub arguments: BTreeMap<String, ScriptValue>,
    /// Host additions, read as `context.extensions`. They cannot shadow the
    /// fields above.
    pub extensions: BTreeMap<String, ScriptValue>,
}

impl Default for CommandContext {
    fn default() -> Self {
        Self {
            version: CONTEXT_VERSION,
            selection: Vec::new(),
            focus: None,
            arguments: BTreeMap::new(),
            extensions: BTreeMap::new(),
        }
    }
}

impl CommandContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_selection<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.selection = ids.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_focus(mut self, kind: impl Into<String>, target: Option<String>) -> Self {
        self.focus = Some(Focus {
            kind: kind.into(),
            target,
        });
        self
    }

    pub fn with_argument(mut self, name: impl Into<String>, value: ScriptValue) -> Self {
        self.arguments.insert(name.into(), value);
        self
    }

    pub fn with_extension(mut self, name: impl Into<String>, value: ScriptValue) -> Self {
        self.extensions.insert(name.into(), value);
        self
    }

    /// The value bound as `context`: `version`, `selection`, `focus`
    /// (`kind`, `target`), `arguments` and `extensions`. An unset focus or
    /// target is absent, which a script reads as `nil`.
    pub fn to_script_value(&self) -> ScriptValue {
        let text = |value: &str| ScriptValue::Str(value.to_string());
        let mut map = BTreeMap::new();
        map.insert("version".into(), ScriptValue::Int(i64::from(self.version)));
        map.insert(
            "selection".into(),
            ScriptValue::List(self.selection.iter().map(|id| text(id)).collect()),
        );
        if let Some(focus) = &self.focus {
            let mut view = BTreeMap::new();
            view.insert("kind".into(), text(&focus.kind));
            if let Some(target) = &focus.target {
                view.insert("target".into(), text(target));
            }
            map.insert("focus".into(), ScriptValue::Map(view));
        }
        map.insert("arguments".into(), ScriptValue::Map(self.arguments.clone()));
        map.insert(
            "extensions".into(),
            ScriptValue::Map(self.extensions.clone()),
        );
        ScriptValue::Map(map)
    }
}
