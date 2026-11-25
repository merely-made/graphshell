// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! Reopen a named linked-data view through an admitted native resident connection.
use mere_ontology::{CITATION, WEB_ANNOTATION};
use mere_overmap::saved_views::SavedViewsClient;
use ostiary::{
    app_admission::{AppId, configured_app_endpoint},
    declaration::{AppDeclaration, NamePattern},
};
use pandect::SessionId;
use std::path::Path;

const USAGE: &str = "Usage: mere-reservoir-view DOMAIN (current|SESSION_UUID) list | open NAME | run NAME LENS_DIRECTORY";

enum Operation<'a> {
    List,
    Open(&'a str),
    Run(&'a str, &'a Path),
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.as_slice() == ["--help"] {
        println!(
            "{USAGE}\nReads the running resident through this command's grant for mere/DOMAIN. Its ordinary explicit route selects a live session and creates an empty one if this served domain has none."
        );
        return Ok(());
    }
    let [domain, session, rest @ ..] = args.as_slice() else {
        return Err(USAGE.into());
    };
    let chosen = if session == "current" {
        None
    } else {
        Some(SessionId(
            uuid::Uuid::parse_str(session).map_err(|e| e.to_string())?,
        ))
    };
    let operation = match rest {
        [cmd] if cmd == "list" => Operation::List,
        [cmd, name] if cmd == "open" => Operation::Open(name),
        [cmd, name, directory] if cmd == "run" => Operation::Run(name, Path::new(directory)),
        _ => return Err(USAGE.into()),
    };
    let declaration = AppDeclaration {
        routes: vec![NamePattern::exact(format!("mere/{domain}"))],
        ..Default::default()
    };
    let mut client = SavedViewsClient::connect_existing(
        &configured_app_endpoint(),
        AppId::new("mere-reservoir-view"),
        declaration,
        domain,
        chosen,
    )
    .await?;
    let output = match operation {
        Operation::List => serde_json::to_string_pretty(
            &serde_json::json!({"session":client.session(),"views":client.list().await?}),
        )
        .map_err(|e| e.to_string())?,
        Operation::Open(name) => client.open(name).await?.to_json()?,
        Operation::Run(name, directory) => {
            let profile = mere_ontology::load("", &[WEB_ANNOTATION, CITATION], Some(directory))?;
            serde_json::to_string_pretty(&client.evaluate(name, &profile).await?)
                .map_err(|e| e.to_string())?
        },
    };
    client.close().await?;
    println!("{output}");
    Ok(())
}
