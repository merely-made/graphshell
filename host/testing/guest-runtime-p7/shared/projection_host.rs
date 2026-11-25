// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The projection guest's host (guest runtime plan, P7): the bindings and the
//! `Host` impl over the fixture server that `app-host`'s test uses, with the
//! WASI the guest's standard library reaches for supplied by `stubs`. It needs
//! no `wasmtime-wasi`, so the same file builds natively and for
//! `wasm32-unknown-unknown`; each crate includes it by path.

use graphshell_endpoint::wit::bindings::{Handler, Session};
use graphshell_endpoint::wit::testkit::{CALLS_BEFORE_REVOCATION, CountdownGate, FixtureEndpoint};
use graphshell_endpoint::wit::{self, WitServer};
use wasmtime::component::{Component, HasSelf, Linker, Resource, ResourceTable};
use wasmtime::{Engine, Store};

/// The host's record of one opened session: the server's handle.
pub struct SessionRep(Session);

wasmtime::component::bindgen!({
    path: [
        "../../../crates/script/wit",
        "../../../crates/script/app-host/guest-projection/wit",
    ],
    world: "mere:projection-test/projection-consumer",
    with: { "mere:script/projection-session.session": SessionRep },
});

use mere::script::projection_session as ps;

pub type Server = WitServer<FixtureEndpoint, CountdownGate>;

/// The fixture server the native test serves the guest from.
pub fn server() -> Server {
    WitServer::new(
        FixtureEndpoint::default(),
        CountdownGate::new(CALLS_BEFORE_REVOCATION),
    )
}

fn wire(f: wit::Failure) -> ps::Failure {
    match f {
        wit::Failure::Revoked => ps::Failure::Revoked,
        wit::Failure::Expired => ps::Failure::Expired,
        wit::Failure::TooLarge(n) => ps::Failure::TooLarge(n),
        wit::Failure::Refused(s) => ps::Failure::Refused(s),
        wit::Failure::Malformed(s) => ps::Failure::Malformed(s),
    }
}

fn notice(n: wit::Notice) -> ps::Notice {
    ps::Notice {
        epoch: n.epoch,
        revision: n.revision,
    }
}

/// The store data: the fixture server and the table of session handles.
pub struct Host {
    pub table: ResourceTable,
    pub server: Server,
}

impl Host {
    fn session(&self, s: &Resource<SessionRep>) -> Session {
        self.table
            .get(s)
            .map(|r| r.0.clone())
            .unwrap_or(Session(Vec::new()))
    }
}

impl ps::Host for Host {
    fn discover(&mut self) -> Result<Vec<u8>, ps::Failure> {
        self.server.discover().map_err(wire)
    }
    fn open(&mut self, request: Vec<u8>) -> Result<(Resource<SessionRep>, Vec<u8>), ps::Failure> {
        let (session, opened) = self.server.open(request).map_err(wire)?;
        let res = self
            .table
            .push(SessionRep(session))
            .map_err(|e| ps::Failure::Refused(e.to_string()))?;
        Ok((res, opened))
    }
}

impl ps::HostSession for Host {
    fn snapshot(&mut self, s: Resource<SessionRep>, ask: Vec<u8>) -> Result<Vec<u8>, ps::Failure> {
        let s = self.session(&s);
        self.server.session_snapshot(&s, ask).map_err(wire)
    }
    fn fetch(&mut self, s: Resource<SessionRep>, address: Vec<u8>) -> Result<Vec<u8>, ps::Failure> {
        let s = self.session(&s);
        self.server.session_fetch(&s, address).map_err(wire)
    }
    fn chunk(
        &mut self,
        s: Resource<SessionRep>,
        address: Vec<u8>,
        offset: u64,
        length: u32,
    ) -> Result<Vec<u8>, ps::Failure> {
        let s = self.session(&s);
        self.server
            .session_chunk(&s, address, offset, length)
            .map_err(wire)
    }
    fn resume(&mut self, s: Resource<SessionRep>, request: Vec<u8>) -> Result<Vec<u8>, ps::Failure> {
        let s = self.session(&s);
        self.server.session_resume(&s, request).map_err(wire)
    }
    fn intent(&mut self, s: Resource<SessionRep>, call: Vec<u8>) -> Result<Vec<u8>, ps::Failure> {
        let s = self.session(&s);
        self.server.session_intent(&s, call).map_err(wire)
    }
    fn suspend(&mut self, s: Resource<SessionRep>) -> Result<(), ps::Failure> {
        let s = self.session(&s);
        self.server.session_suspend(&s).map_err(wire)
    }
    fn take_notice(&mut self, s: Resource<SessionRep>) -> Option<ps::Notice> {
        let s = self.session(&s);
        self.server.session_take_notice(&s).map(notice)
    }
    fn wait_notice(&mut self, s: Resource<SessionRep>) -> Result<ps::Notice, ps::Failure> {
        let s = self.session(&s);
        self.server
            .session_wait_notice(&s)
            .map(notice)
            .map_err(wire)
    }
    fn drop(&mut self, s: Resource<SessionRep>) -> wasmtime::Result<()> {
        let rep = self.table.delete(s)?;
        self.server.drop_session(rep.0);
        Ok(())
    }
}

/// What a projection run produced.
pub struct Projection {
    pub transcript: Vec<(String, String)>,
    /// Sessions the guest closed, and sessions left open, as the server counted them.
    pub closed_sessions: usize,
    pub open_sessions: usize,
    /// What the guest wrote to its standard error (a panic message, say).
    pub stderr: String,
    /// Each WASI function the guest called, with its count.
    pub wasi_calls: std::collections::BTreeMap<&'static str, u32>,
}

/// A guest linked and instantiated, not yet run: timing splits the two.
pub struct Started {
    store: Store<Host>,
    guest: ProjectionConsumer,
    output: crate::stubs::StubOutput,
}

/// Links the guest to the fixture server and the WASI stubs and instantiates it.
pub fn start(engine: &Engine, component: &Component) -> wasmtime::Result<Started> {
    let mut linker = Linker::new(engine);
    let output = crate::stubs::add_wasi_stubs(&mut linker)?;
    ps::add_to_linker::<Host, HasSelf<Host>>(&mut linker, |h| h)?;
    let mut store = Store::new(
        engine,
        Host {
            table: ResourceTable::new(),
            server: server(),
        },
    );
    let guest = ProjectionConsumer::instantiate(&mut store, component, &linker)?;
    Ok(Started {
        store,
        guest,
        output,
    })
}

impl Started {
    /// Runs the guest's script once.
    pub fn run(mut self) -> wasmtime::Result<Projection> {
        let transcript = self.guest.call_run(&mut self.store)?;
        let host = self.store.data();
        Ok(Projection {
            transcript,
            closed_sessions: host.server.gate.closed.len(),
            open_sessions: host.server.open_sessions(),
            stderr: self.output.stderr(),
            wasi_calls: self.output.calls(),
        })
    }
}
