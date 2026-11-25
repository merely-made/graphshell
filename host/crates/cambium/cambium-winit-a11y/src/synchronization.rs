// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Tree publication; queued requests retain the preceding namespace.

use super::*;

impl A11yHost {
    /// Project the current layout into an AccessKit tree (with each leaf's own
    /// semantics), install it on the first call — revealing `window`, which must
    /// have been created hidden so the adapter attaches first — and update it
    /// after. Returns the screen reader's Click and Focus requests, in request
    /// order and still typed, for the caller to route each through the matching
    /// path (activation for `Click`, focus for `Focus`).
    ///
    /// `focus` is the app's currently-focused DOM node's opaque id (from
    /// `LayoutDom::opaque_id`), used as the tree's focus when it is really in the
    /// tree, so a stale id never points the reader at nothing.
    pub(super) fn sync_inner(
        &mut self,
        owner: &cambium_rootstock::A11yOwner,
        dom: &cambium_rootstock::WindowDom<'_>,
        layout: &cambium_rootstock::OwnedLayout,
        leaves: &LeafRegistry<u64>,
        producers: &ProducerRegistry,
        focus: Option<u64>,
        layout_scale: f64,
    ) -> A11ySync {
        let (window, requests) = match self.sync_prelude(owner) {
            Ok(ready) => ready,
            Err(sync) => return sync,
        };
        let projection =
            cambium_rootstock::project_semantics(dom, layout, leaves, producers, focus);
        let bindings = projection.publication_envelopes(owner, dom, leaves, producers);
        let issues = projection.producer_issues.len();
        let publish = Publish {
            window: &window,
            requests,
            contribution_issues: issues,
            layout_scale,
            retain: None,
        };
        self.publish_whole(publish, projection.tree, bindings)
    }

    /// Retire everything an ended owner or a missing window cannot keep, and
    /// resolve queued events through the publication they could have seen.
    pub(super) fn sync_prelude(
        &mut self,
        owner: &cambium_rootstock::A11yOwner,
    ) -> Result<(std::sync::Arc<dyn Window>, Vec<A11yRequest>), A11ySync> {
        if !owner.is_live() {
            self.bridge.drain_actions();
            self.clear_publication();
            return Err(A11ySync {
                requests: Vec::new(),
                publication: A11yPublication::skipped(
                    self.adapter_state(),
                    A11yPublicationSkip::OwnerEnded,
                ),
            });
        }
        let Some(window) = self.window.clone() else {
            // No window yet: nothing to install against, and no reader to ask.
            return Err(A11ySync {
                requests: Vec::new(),
                publication: A11yPublication::skipped(
                    A11yAdapterState::Detached,
                    A11yPublicationSkip::NoWindow,
                ),
            });
        };
        // Resolve queued events before reading contributors or replacing the
        // published maps.
        let requests = self.drain_native_actions(owner);
        Ok((window, requests))
    }

    pub(super) fn clear_publication(&mut self) {
        self.requests.clear();
        self.envelopes.clear();
        self.namespace.clear();
        self.native = None;
    }

    /// Publish a whole composed tree. `publish.retain` keeps the native tree
    /// for later partial publications, stamped with that composition.
    pub(super) fn publish_whole(
        &mut self,
        publish: Publish<'_>,
        mut tree: TreeUpdate,
        bindings: Result<
            cambium_rootstock::SemanticPublication,
            cambium_rootstock::PublicationIssue,
        >,
    ) -> A11ySync {
        let Publish {
            window,
            requests,
            contribution_issues,
            layout_scale,
            retain,
        } = publish;
        if let Some((_, node)) = tree
            .tree
            .as_ref()
            .map(|info| info.root)
            .and_then(|root| tree.nodes.iter_mut().find(|(id, _)| *id == root))
        {
            scale_root(node, layout_scale);
        }
        let publication = bindings
            .map_err(|issue| {
                (
                    A11yPublicationRefusal::InvalidBindings,
                    format!("{issue:?}"),
                )
            })
            .and_then(|bindings| {
                self.namespace
                    .prepare(tree, bindings)
                    .map_err(|issue| (A11yPublicationRefusal::InvalidTree, format!("{issue:?}")))
            });
        let publication::Prepared {
            tree,
            namespace,
            requests: next,
            envelopes,
        } = match publication {
            Ok(candidate) => candidate,
            Err((reason, issue)) => {
                self.clear_publication();
                eprintln!("[cambium-winit] accessibility publication refused: {issue}");
                // Match install failure: a rejected accessibility tree must not
                // leave the application hidden. A later sync may retry.
                window.set_visible(true);
                return A11ySync {
                    requests,
                    publication: A11yPublication::refused(self.adapter_state(), reason),
                };
            },
        };
        let node_count = tree.nodes.len();
        let native = retain.map(|sequence| {
            crate::retained_sync::NativeTree::new(tree.clone(), sequence, layout_scale)
        });
        let mut outcome = A11yPublicationOutcome::Submitted {
            nodes: node_count,
            contribution_issues,
        };
        if !self.installed {
            match crate::forest_publisher::install_native(&mut self.bridge, window.as_ref(), tree) {
                Ok(()) => {
                    self.installed = true;
                    self.requests = next;
                    self.envelopes = envelopes;
                    self.namespace = namespace;
                    self.native = native;
                    eprintln!(
                        "[cambium-winit] accessibility {:?}, {node_count} nodes projected",
                        self.bridge.status()
                    );
                },
                Err(e) => {
                    self.clear_publication();
                    outcome = A11yPublicationOutcome::Refused {
                        reason: A11yPublicationRefusal::AdapterInstall,
                    };
                    eprintln!("[cambium-winit] accessibility install failed: {e}");
                },
            }
            // Keep the app usable on adapter failure; retry installation on the
            // next sync rather than claiming that a failed install succeeded.
            window.set_visible(true);
        } else if let Err(error) = crate::forest_publisher::submit_root(&mut self.bridge, tree) {
            self.clear_publication();
            outcome = A11yPublicationOutcome::Refused {
                reason: A11yPublicationRefusal::InvalidTree,
            };
            eprintln!("[cambium-winit] accessibility publication refused: {error}");
        } else {
            self.requests = next;
            self.envelopes = envelopes;
            self.namespace = namespace;
            self.native = native;
        }
        A11ySync {
            requests,
            publication: A11yPublication {
                adapter: self.adapter_state(),
                outcome,
            },
        }
    }
}

/// One publication's fixed inputs.
pub(super) struct Publish<'a> {
    pub window: &'a std::sync::Arc<dyn Window>,
    pub requests: Vec<A11yRequest>,
    pub contribution_issues: usize,
    pub layout_scale: f64,
    /// The composition sequence to retain the native tree under, if any.
    pub retain: Option<u64>,
}

/// Stamp the layout scale on the root, over its own parent-relative translate.
pub(super) fn scale_root(node: &mut accesskit::Node, layout_scale: f64) {
    let own = node.transform().copied().unwrap_or(Affine::IDENTITY);
    node.set_transform(Affine::scale(layout_scale) * own);
}
