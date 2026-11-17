/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::cell::Cell;
use std::rc::Rc;

use cambium_rootstock::A11yOwner;
use document_session_api::session_engine::{DocumentSession, SessionEngine, SessionSpawnRequest};
use document_session_api::{
    DocumentA11yAction, DocumentA11yActionData, DocumentA11yActionRequest, DocumentA11yProjection,
};
use genet_documents::{LiverySessionEngine, LocalFetcher};
use netrender::Scene;
use uxtree::tree_id_for_path;

use super::{Address, Envelope, Owner, Provenance, Publication, Stability};

pub(super) struct Session {
    document: Box<dyn DocumentSession<Scene>>,
    live: Rc<Cell<bool>>,
    initial_revision: u64,
    freeze_scope: bool,
}

impl Session {
    pub(super) fn new(freeze_scope: bool) -> Self {
        let mut document = LiverySessionEngine::new(LocalFetcher)
            .spawn(
                &SessionSpawnRequest::new("https://p0.invalid/document")
                    .with_body("<input id=document-value aria-label='Document value' value=before>")
                    .with_viewport(200, 120),
            )
            .unwrap();
        let _ = document.frame(200, 120);
        let initial_revision = document.accessibility_projection().unwrap().revision();
        Self {
            document,
            live: Rc::new(Cell::new(true)),
            initial_revision,
            freeze_scope,
        }
    }

    fn projection(&self) -> DocumentA11yProjection {
        self.document
            .accessibility_projection()
            .expect("the document supplies its projection")
    }

    pub(super) fn revision(&self) -> u64 {
        self.projection().revision()
    }

    pub(super) fn change_value(&mut self) {
        let projection = self.projection();
        let node = projection
            .nodes()
            .iter()
            .find(|node| node.name.as_deref() == Some("Document value"))
            .unwrap();
        let old = DocumentA11yActionRequest {
            revision: projection.revision(),
            target: node.id,
            action: DocumentA11yAction::SetValue,
            data: Some(DocumentA11yActionData::Value("after".into())),
        };
        assert!(self.document.dispatch_accessibility_action(&old));
        let _ = self.document.frame(200, 120);
        assert_ne!(self.revision(), old.revision);
        assert!(
            !self.document.dispatch_accessibility_action(&old),
            "the engine itself rejects its stale revision"
        );
        assert_eq!(
            self.projection().node(old.target).unwrap().value.as_deref(),
            Some("after")
        );
    }
}

impl Owner for Session {
    fn instance(&self) -> accesskit::TreeId {
        tree_id_for_path("p0/document")
    }

    fn publish(&self) -> Publication {
        let projection = self.projection();
        let revision = if self.freeze_scope {
            self.initial_revision
        } else {
            projection.revision()
        };
        let envelopes = projection
            .nodes()
            .iter()
            .map(|node| Envelope {
                address: Address {
                    instance: self.instance(),
                    local: accesskit::NodeId(node.id.get()),
                },
                stability: Stability::Scoped(revision),
                provenance: Provenance {
                    owner: A11yOwner::new(&self.live),
                    request: None,
                    child: None,
                    identity: None,
                },
                action_ids: Vec::new(),
                domain_ref: None,
                // The neutral projection does not export authored DOM keys.
                authored_key: None,
            })
            .collect();
        let mut update = genet_render::accesskit_tree_from_projection(projection);
        update.tree_id = self.instance();
        Publication { update, envelopes }
    }

    fn current(&self, observation: &Envelope) -> bool {
        observation.provenance.owner.is_live()
            && observation.provenance.owner == A11yOwner::new(&self.live)
            && observation.address.instance == self.instance()
            && observation.stability == Stability::Scoped(self.revision())
            && self
                .projection()
                .nodes()
                .iter()
                .any(|node| node.id.get() == observation.address.local.0)
    }

    fn invoke(&mut self, _: &Envelope, _: &str) -> bool {
        // A document exposes action kinds, but advertises no command IDs.
        false
    }
}
