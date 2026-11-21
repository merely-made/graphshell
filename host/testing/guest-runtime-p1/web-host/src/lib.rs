// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The fixture projection server, callable from a page (guest runtime plan, P1).
//!
//! In a web build the host is Rust compiled to wasm and the guest is a
//! component the browser runs itself. This crate is the host half: the same
//! `WitServer` the native test uses, with each call exposed in the value
//! shapes the converted guest's glue expects. A session crosses as its opaque
//! handle bytes, so no table lives here.

use graphshell_endpoint::wit::bindings::{Handler, Session};
use graphshell_endpoint::wit::testkit::{CALLS_BEFORE_REVOCATION, CountdownGate, FixtureEndpoint};
use graphshell_endpoint::wit::{Failure, Notice, WitServer};
use js_sys::{Array, BigInt, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

/// A WIT variant as the glue reads one: `{ tag, val }`.
fn variant(tag: &str, val: Option<JsValue>) -> JsValue {
    let object = Object::new();
    let _ = Reflect::set(&object, &"tag".into(), &tag.into());
    if let Some(val) = val {
        let _ = Reflect::set(&object, &"val".into(), &val);
    }
    object.into()
}

/// A `failure`, thrown to the glue, which reads a thrown value as the error
/// arm of a `result`.
fn failure(failure: Failure) -> JsValue {
    match failure {
        Failure::Revoked => variant("revoked", None),
        Failure::Expired => variant("expired", None),
        Failure::TooLarge(size) => variant("too-large", Some(BigInt::from(size).into())),
        Failure::Refused(why) => variant("refused", Some(why.into())),
        Failure::Malformed(why) => variant("malformed", Some(why.into())),
    }
}

fn notice(notice: Notice) -> JsValue {
    let object = Object::new();
    let _ = Reflect::set(&object, &"epoch".into(), &BigInt::from(notice.epoch).into());
    let _ = Reflect::set(&object, &"revision".into(), &BigInt::from(notice.revision).into());
    object.into()
}

#[wasm_bindgen]
pub struct FixtureHost {
    server: WitServer<FixtureEndpoint, CountdownGate>,
}

impl Default for FixtureHost {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl FixtureHost {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            server: WitServer::new(
                FixtureEndpoint::default(),
                CountdownGate::new(CALLS_BEFORE_REVOCATION),
            ),
        }
    }

    pub fn discover(&mut self) -> Result<Vec<u8>, JsValue> {
        self.server.discover().map_err(failure)
    }

    /// `[session handle, opened payload]`.
    pub fn open(&mut self, request: Vec<u8>) -> Result<Array, JsValue> {
        let (session, opened) = self.server.open(request).map_err(failure)?;
        let pair = Array::new();
        pair.push(&Uint8Array::from(session.0.as_slice()));
        pair.push(&Uint8Array::from(opened.as_slice()));
        Ok(pair)
    }

    pub fn snapshot(&mut self, session: Vec<u8>, ask: Vec<u8>) -> Result<Vec<u8>, JsValue> {
        self.server.session_snapshot(&Session(session), ask).map_err(failure)
    }

    pub fn fetch(&mut self, session: Vec<u8>, address: Vec<u8>) -> Result<Vec<u8>, JsValue> {
        self.server.session_fetch(&Session(session), address).map_err(failure)
    }

    pub fn chunk(
        &mut self,
        session: Vec<u8>,
        address: Vec<u8>,
        offset: u64,
        length: u32,
    ) -> Result<Vec<u8>, JsValue> {
        self.server
            .session_chunk(&Session(session), address, offset, length)
            .map_err(failure)
    }

    pub fn resume(&mut self, session: Vec<u8>, epoch: u64, revision: u64) -> Result<Vec<u8>, JsValue> {
        self.server
            .session_resume(&Session(session), epoch, revision)
            .map_err(failure)
    }

    pub fn intent(&mut self, session: Vec<u8>, call: Vec<u8>) -> Result<Vec<u8>, JsValue> {
        self.server.session_intent(&Session(session), call).map_err(failure)
    }

    pub fn suspend(&mut self, session: Vec<u8>) -> Result<(), JsValue> {
        self.server.session_suspend(&Session(session)).map_err(failure)
    }

    /// A notice, or `undefined` when none is waiting.
    #[wasm_bindgen(js_name = takeNotice)]
    pub fn take_notice(&mut self, session: Vec<u8>) -> JsValue {
        self.server
            .session_take_notice(&Session(session))
            .map_or(JsValue::UNDEFINED, notice)
    }

    #[wasm_bindgen(js_name = waitNotice)]
    pub fn wait_notice(&mut self, session: Vec<u8>) -> Result<JsValue, JsValue> {
        self.server
            .session_wait_notice(&Session(session))
            .map(notice)
            .map_err(failure)
    }

    #[wasm_bindgen(js_name = dropSession)]
    pub fn drop_session(&mut self, session: Vec<u8>) {
        self.server.drop_session(Session(session));
    }

    /// Sessions the guest closed, as the gate recorded them.
    #[wasm_bindgen(js_name = closedSessions)]
    pub fn closed_sessions(&self) -> usize {
        self.server.gate.closed.len()
    }

    #[wasm_bindgen(js_name = openSessions)]
    pub fn open_sessions(&self) -> usize {
        self.server.open_sessions()
    }
}
