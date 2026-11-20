use super::*;
use mere_surface_api::settings::{
    SettingMovement, SettingMutability, SettingScope, SettingSecurity,
};

fn snapshot() -> SettingRowSnapshot {
    let spec = SettingSpec {
        id: "unsettled.setting".into(),
        label: "Unsettled setting".into(),
        scope: SettingScope::Application,
        movement: SettingMovement::LocalOnly,
        mutability: SettingMutability::Live,
        security: SettingSecurity::Ordinary,
        control: SettingControl::Text,
        value: SettingValue::Text("a".into()),
    };
    SettingRowSession::new(spec).snapshot()
}

#[test]
fn unsettled_is_dirty_or_pending() {
    for (dirty, pending, unsettled) in [
        (false, None, false),
        (true, None, true),
        (false, Some(1), true),
        (true, Some(1), true),
    ] {
        let row = SettingRowSnapshot {
            dirty,
            pending_generation: pending,
            ..snapshot()
        };
        assert_eq!(
            row.is_unsettled(),
            unsettled,
            "dirty {dirty}, pending {pending:?}"
        );
    }
}
