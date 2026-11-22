// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! `mere`: query linked data on this machine, or serve one granted query to an
//! invited reader (graphshell:mere-cli plan).

#[cfg(feature = "webrtc")]
mod browser;
mod carriers;
mod files;
mod identity;
mod local;
mod serve;

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgGroup, Parser, Subcommand};
use graphshell_host::invitation::Contact;

use crate::files::{InviteFile, read_json, read_query, write_new};

#[derive(Parser)]
#[command(name = "mere", version, about = "Query linked data on this machine or an invited peer")]
struct Cli {
    /// A file holding a 32-byte hex seed to speak as, in place of the custodian.
    #[arg(long, global = true, hide = true)]
    identity_seed: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a SPARQL query and print its rows as JSON arrays in column order.
    #[command(group(ArgGroup::new("source").args(["graph", "invite"]).required(true)))]
    Query {
        /// A local RDF dataset.
        #[arg(long)]
        graph: Option<PathBuf>,
        /// An invitation from a peer who serves the query.
        #[arg(long)]
        invite: Option<PathBuf>,
        /// A UTF-8 SPARQL query file.
        #[arg(long)]
        query: PathBuf,
        /// The loopback I2P SAM bridge (default 127.0.0.1:7656).
        #[arg(long, requires = "invite")]
        sam: Option<SocketAddr>,
    },
    /// Write this persona's contact, for an owner to invite.
    Contact {
        #[arg(long)]
        out: PathBuf,
    },
    /// Invite a contact to one query over a graph, and serve it until stopped.
    Serve(serve::ServeArgs),
    /// Revoke an invitation this persona issued.
    Revoke {
        /// The invitation file `serve` wrote.
        #[arg(long)]
        invite: PathBuf,
        /// The directory the serve watches (default: `<invite>.revocations`).
        #[arg(long)]
        revocations: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let seed = cli.identity_seed.as_deref();
    match cli.command {
        Command::Query {
            graph,
            invite,
            query,
            sam,
        } => {
            let query = read_query(&query)?;
            let rows = match (graph, invite) {
                (Some(path), None) => local::query(&local::dataset(&path)?, &query)?,
                (None, Some(path)) => {
                    let invite: InviteFile = read_json(&path)?;
                    carriers::ask(&identity::identity(seed)?, &invite, query, sam).await?
                },
                _ => unreachable!("clap requires exactly one source"),
            };
            serde_json::to_writer(std::io::stdout().lock(), &rows)?;
            println!();
        },
        Command::Contact { out } => {
            let contact = Contact::create(identity::identity(seed)?.as_ref(), &identity::TERMS)
                .map_err(anyhow::Error::msg)?;
            write_new(&out, &contact)?;
        },
        Command::Serve(args) => {
            let identity = identity::identity(seed)?;
            tokio::select! {
                result = serve::serve(args, identity) => result?,
                signal = tokio::signal::ctrl_c() => signal?,
            }
        },
        Command::Revoke {
            invite,
            revocations,
        } => {
            let file: InviteFile = read_json(&invite)?;
            let revocation = file
                .invitation
                .revoke(identity::identity(seed)?.as_ref(), carriers::now_ms())
                .map_err(anyhow::Error::msg)?;
            let dir = revocations.unwrap_or_else(|| files::revocations_beside(&invite));
            std::fs::create_dir_all(&dir)?;
            let id = file.invitation.grant.certificate.id().0;
            let name = format!("{}.json", id.iter().map(|b| format!("{b:02x}")).collect::<String>());
            write_new(&dir.join(name), &revocation)?;
            eprintln!("Revoked. A serve watching {} stops this reader.", dir.display());
        },
    }
    Ok(())
}
