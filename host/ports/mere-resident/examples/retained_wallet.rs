// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0
//! Receipt helper, isolated profile only: the owner's own step before the
//! resident starts. Stages the wallet and a key epoch for the retained persona
//! under DATA_ROOT; nothing is created implicitly by the resident.
//! Usage: `retained_wallet DATA_ROOT PERSONA_UUID`.
fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("data root"));
    let persona: uuid::Uuid = std::env::args().nth(2).expect("persona uuid").parse().expect("a uuid");
    // The resident opens this wallet in its own process through the OS slot.
    // Examples build with castellan's `test-support`, which has no implicit
    // DPAPI, so the slot is named here. Elsewhere the store is the platform's
    // installed one, or the wallet is refused (custody D5).
    #[cfg(windows)]
    castellan::custody::os_kek::install_os_kek_store(&root, std::sync::Arc::new(castellan::custody::os_kek::Dpapi));
    castellan::custody::wallet::ensure_wallet_state(&root, personae::PersonaId::from_uuid(persona), "Retained receipt")
        .expect("the wallet is staged");
    println!("staged");
}
