// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The inspector's presentation of a reported validation finding and its evidence.

use crate::{
    AnyView, DetailRow, DetailSection, GenetCtx, GenetElement, OptionalAction, View, button,
    detail_panel, el,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationFinding {
    pub focus_node: String,
    pub path: Option<String>,
    pub severity: String,
    pub message: Option<String>,
}

/// Evidence offered by the host for this finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationEvidence<Id> {
    pub id: Id,
    pub detail: DetailSection,
}

/// Show a finding alongside its evidence. A single match offers a direct
/// action on the finding. Multiple matches offer a choice of evidence.
///
/// The host resolves the chosen handle within its current source scope and
/// controls which evidence remains selected.
pub fn validation_finding_panel<State, Action, Id, Output, Open>(
    finding: &ValidationFinding,
    evidence: &[ValidationEvidence<Id>],
    selected: Option<&Id>,
    open: Open,
) -> impl View<State, Action, GenetCtx, Element = GenetElement> + use<State, Action, Id, Output, Open>
where
    State: 'static,
    Action: 'static,
    Id: Clone + PartialEq + 'static,
    Output: OptionalAction<Action>,
    Open: Fn(&mut State, Id) -> Output + Clone + 'static,
{
    type Child<State, Action> = Box<dyn AnyView<State, Action, GenetCtx, GenetElement>>;

    let mut rows = vec![DetailRow::new("Focus node", &finding.focus_node)];
    if let Some(path) = &finding.path {
        rows.push(DetailRow::new("Path", path));
    }
    rows.push(DetailRow::new("Severity", &finding.severity));
    if let Some(message) = &finding.message {
        rows.push(DetailRow::new("Message", message));
    }

    let mut children: Vec<Child<State, Action>> =
        vec![Box::new(detail_panel(&[DetailSection::new(
            "Validation finding",
            rows,
        )]))];
    if let [only] = evidence {
        let id = only.id.clone();
        let open = open.clone();
        children.push(Box::new(
            button("Open finding evidence", move |state, _| {
                open(state, id.clone())
            })
            .attr("class", "shell-control validation-finding-open"),
        ));
    }
    for item in evidence {
        let mut section: Vec<Child<State, Action>> =
            vec![Box::new(detail_panel(std::slice::from_ref(&item.detail)))];
        if evidence.len() > 1 {
            let id = item.id.clone();
            let open = open.clone();
            section.push(Box::new(
                button(format!("Open {}", item.detail.title), move |state, _| {
                    open(state, id.clone())
                })
                .attr("class", "shell-control validation-finding-open"),
            ));
        }
        let current = selected == Some(&item.id);
        children.push(Box::new(
            el("div", section)
                .attr("class", "validation-finding-evidence")
                .attr("aria-current", if current { "true" } else { "false" })
                .attr(
                    "style",
                    if current {
                        "border-inline-start: 2px solid currentColor;"
                    } else {
                        ""
                    },
                ),
        ));
    }
    el("section", children)
        .attr("class", "validation-finding")
        .attr("aria-label", "Validation finding")
}

#[cfg(test)]
mod tests;
