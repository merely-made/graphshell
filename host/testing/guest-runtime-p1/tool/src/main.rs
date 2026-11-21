// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Native tool for the guest runtime probe (guest runtime plan, P0 and P1).
//!
//! `reference <out.json>` runs the shared call script against the fixture
//! server over the native pipe route and writes its transcript: the record
//! every other runtime is compared with.
//!
//! `convert <component.wasm> <out dir> <name>` turns a component into the
//! modules and glue a browser loads, using the converter inside `jco` from
//! Rust, so the web build needs no Node tool.

use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;
use std::{env, fs, thread};

use graphshell_endpoint::wit::testkit::{
    CALLS_BEFORE_REVOCATION, CountdownGate, FixtureEndpoint, assert_outcomes, run,
};
use graphshell_endpoint::wit::{Calls, MAX_FRAME, WitCarrier, WitServer, serve};
use js_component_bindgen::{InstantiationMode, TranspileOpts, transpile};

/// Where the converted guest looks for each WASI interface: local stubs,
/// named here so nothing is fetched from a package registry.
const WASI_STUB_MODULE: &str = "./wasi-stubs.js";

fn reference(out: &Path) -> Result<(), String> {
    let (client, end) = script_wire::pipe::pair(MAX_FRAME).map_err(|e| e.to_string())?;
    let native = thread::spawn(move || {
        let mut server = WitServer::new(
            FixtureEndpoint::default(),
            CountdownGate::new(CALLS_BEFORE_REVOCATION),
        );
        serve(&mut server, end)
    });
    let transcript = run(&mut WitCarrier::new(Calls::new(client)));
    native
        .join()
        .map_err(|_| "the native server thread panicked".to_string())?
        .map_err(|e| e.to_string())?;
    assert_outcomes(&transcript);
    let json = serde_json::to_string_pretty(&transcript).map_err(|e| e.to_string())?;
    fs::write(out, json).map_err(|e| e.to_string())?;
    println!("{} calls written to {}", transcript.len(), out.display());
    Ok(())
}

fn convert(component: &Path, out: &Path, name: &str) -> Result<(), String> {
    let bytes = fs::read(component).map_err(|e| format!("{}: {e}", component.display()))?;
    let mut map = HashMap::new();
    map.insert("wasi:*".to_string(), format!("{WASI_STUB_MODULE}#*"));
    let opts = TranspileOpts {
        name: name.to_string(),
        no_typescript: true,
        // The page supplies the imports and the compiled modules itself.
        instantiation_mode: Some(InstantiationMode::Async),
        map: Some(map),
        nodejs_compat_disabled: true,
        ..TranspileOpts::default()
    };
    let converted = transpile(&bytes, opts).map_err(|e| format!("{e:#}"))?;
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut shipped = 0;
    for (file, contents) in &converted.files {
        let path = out.join(file);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, contents).map_err(|e| e.to_string())?;
        println!("{:>9} bytes  {file}", contents.len());
        shipped += contents.len();
    }
    println!("{shipped:>9} bytes  in {} files", converted.files.len());
    println!("imports:");
    for import in &converted.imports {
        println!("  {import}");
    }
    println!("exports:");
    for (export, _) in &converted.exports {
        println!("  {export}");
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["reference", out] => reference(Path::new(out)),
        ["convert", component, out, name] => convert(Path::new(component), Path::new(out), name),
        _ => Err("usage: reference <out.json> | convert <component.wasm> <out dir> <name>".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        },
    }
}
