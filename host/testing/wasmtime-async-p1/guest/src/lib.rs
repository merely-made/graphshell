//! Sync guest for the Wasmtime-45 fiber-async (Model A) maturity probe.
//!
//! `run` is an ordinary sync export; it calls the sync `fetch` import twice with
//! no async coloring. The asyncness is entirely host-side: the host implements
//! `fetch` as a Rust `async fn`, and `Config::async_support` suspends this
//! component's fiber during the host's await. From the guest's view, `fetch`
//! just returns a value — exactly the no-async-coloring script ergonomics we want.

wit_bindgen::generate!({
    path: "../wit",
    world: "async-probe",
});

use crate::probe::suspend::host_io::fetch;

struct Component;

impl Guest for Component {
    fn run(input: String) -> String {
        let a = fetch(&format!("{input}-A"));
        let b = fetch(&format!("{input}-B"));
        format!("guest run({input}): got [{a}] then [{b}]")
    }
}

export!(Component);
