// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Authored context scenes and command presentation choices.
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{ProjectionDefinition, ValidationIssue};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextTrigger {
    Hover,
    Focus,
    Selection,
    Request,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    Node,
    Relation,
    Field,
    Selection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextPlacement {
    Beside,
    Overlay,
    InPlace,
    Pane,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandChoice {
    /// Stable identifier in the host's command disclosure.
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextScene {
    pub id: String,
    pub trigger: ContextTrigger,
    pub subject: SubjectKind,
    /// The host's registered contextual dataset binding.
    pub input: String,
    pub placement: ContextPlacement,
    /// The child definition's source and source revision bind to the live input.
    pub recipe: Box<ProjectionDefinition>,
    /// Present means a curated list, in order. An empty list shows no commands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commands: Option<Vec<CommandChoice>>,
}

pub fn context_issues(scenes: &[ContextScene]) -> Vec<ValidationIssue> {
    let mut errors = Vec::new();
    let mut ids = BTreeSet::new();
    for scene in scenes {
        if scene.id.trim().is_empty() || !ids.insert(&scene.id) {
            errors.push(ValidationIssue::error(
                "marks.contexts",
                "context ids must be present and unique",
            ));
        }
        if scene.input.trim().is_empty() {
            errors.push(ValidationIssue::error(
                "marks.contexts",
                "a context needs an input binding",
            ));
        }
        if let Some(commands) = &scene.commands {
            let mut ids = BTreeSet::new();
            if commands
                .iter()
                .any(|c| c.id.trim().is_empty() || !ids.insert(&c.id))
            {
                errors.push(ValidationIssue::error(
                    "marks.contexts",
                    "command choices must have unique, nonempty ids",
                ));
            }
        }
    }
    errors
}

/// Subject kinds that have a hover context and lack a touch-accessible trigger.
pub fn hover_only_subjects(scenes: &[ContextScene]) -> Vec<SubjectKind> {
    [
        SubjectKind::Node,
        SubjectKind::Relation,
        SubjectKind::Field,
        SubjectKind::Selection,
    ]
    .into_iter()
    .filter(|kind| {
        let mut entries = scenes
            .iter()
            .filter(|scene| scene.subject == *kind)
            .peekable();
        entries.peek().is_some() && entries.all(|scene| scene.trigger == ContextTrigger::Hover)
    })
    .collect()
}

#[cfg(feature = "toml")]
pub fn contexts_from_toml(input: &str) -> Result<Vec<ContextScene>, toml::de::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Authored {
        #[serde(default)]
        contexts: Vec<ContextScene>,
    }
    toml::from_str::<Authored>(input).map(|value| value.contexts)
}

#[cfg(feature = "toml")]
pub fn contexts_to_toml(contexts: &[ContextScene]) -> Result<String, toml::ser::Error> {
    #[derive(Serialize)]
    struct Authored<'a> {
        contexts: &'a [ContextScene],
    }
    toml::to_string_pretty(&Authored { contexts })
}

#[cfg(all(test, feature = "toml"))]
mod tests {
    use super::*;

    #[test]
    fn authored_command_choices_round_trip_with_the_child_definition() {
        let context = ContextScene {
            id: "commands".into(),
            trigger: ContextTrigger::Hover,
            subject: SubjectKind::Node,
            input: "commands".into(),
            placement: ContextPlacement::Overlay,
            recipe: Box::new(crate::tests::valid_draft().to_definition().unwrap()),
            commands: Some(vec![
                CommandChoice {
                    id: "host:fit".into(),
                    label: Some("Frame all".into()),
                    shortcut: Some("Alt+F".into()),
                },
                CommandChoice {
                    id: "applet:open-address".into(),
                    label: None,
                    shortcut: None,
                },
            ]),
        };
        let mut scenes = vec![context.clone()];
        let text = contexts_to_toml(&scenes).unwrap();
        assert_eq!(contexts_from_toml(&text).unwrap(), scenes);
        assert_eq!(hover_only_subjects(&scenes), vec![SubjectKind::Node]);
        scenes.push(ContextScene {
            id: "request".into(),
            trigger: ContextTrigger::Request,
            ..context
        });
        assert!(hover_only_subjects(&scenes).is_empty());
    }
}
