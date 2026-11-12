use super::*;
use genet_scripted_dom::ScriptedDom;
use layout_dom_api::{LayoutDomMut, QualName};

fn qual(local: &str) -> QualName {
    QualName::new(None, Namespace::from(""), LocalName::from(local))
}

#[test]
fn native_control_lookup_uses_html_button_semantics_and_accessible_name() {
    let mut dom = ScriptedDom::new();
    let root = dom.document();
    let minimize = dom.create_element(qual("button"));
    dom.set_attribute(minimize, qual("aria-label"), "Minimize");
    dom.append_child(root, minimize);
    let maximize = dom.create_element(qual("button"));
    dom.set_attribute(maximize, qual("aria-label"), "Maximize");
    dom.append_child(root, maximize);

    assert_eq!(window_control_node(&dom, root, "Maximize"), Some(maximize));
    assert_eq!(window_control_node(&dom, root, "Agrandir"), None);
}

#[test]
fn unknown_regions_do_not_drag() {
    assert_eq!(AppRegion::parse("drag"), AppRegion::Drag);
    assert_eq!(AppRegion::parse(" drag "), AppRegion::Drag);
    assert_eq!(AppRegion::parse("no-drag"), AppRegion::NoDrag);
    assert_eq!(AppRegion::parse("inherit"), AppRegion::NoDrag);
    assert_eq!(AppRegion::parse(""), AppRegion::NoDrag);
}

#[test]
fn commands_queue_and_drain_once() {
    let commands = WindowCommands::new();
    assert!(commands.is_empty());
    commands.minimize();
    commands.close();

    assert_eq!(
        commands.drain(),
        vec![WindowCommand::Minimize, WindowCommand::Close]
    );
    assert!(commands.is_empty(), "a drain empties the queue");
    assert!(commands.drain().is_empty());
}

#[test]
fn a_clone_shares_the_queue() {
    // The whole point: the application stores one clone in its state and
    // the host keeps another.
    let host_side = WindowCommands::new();
    let app_side = host_side.clone();
    app_side.toggle_maximize();
    assert_eq!(host_side.drain(), vec![WindowCommand::ToggleMaximize]);
}

#[test]
fn double_click_needs_both_time_and_proximity() {
    let base = cambium_rootstock::Instant::now();
    let soon = base + std::time::Duration::from_millis(120);
    let late = base + std::time::Duration::from_millis(900);

    let mut cadence = ClickCadence::new();
    assert!(!cadence.press((10.0, 10.0), base), "one press is not two");
    assert!(cadence.press((11.0, 10.0), soon), "close in time and space");

    let mut cadence = ClickCadence::new();
    cadence.press((10.0, 10.0), base);
    assert!(!cadence.press((10.0, 10.0), late), "too slow");

    let mut cadence = ClickCadence::new();
    cadence.press((10.0, 10.0), base);
    assert!(!cadence.press((40.0, 10.0), soon), "too far");
}

#[test]
fn a_double_click_does_not_chain_into_a_third() {
    let base = cambium_rootstock::Instant::now();
    let mut cadence = ClickCadence::new();
    cadence.press((0.0, 0.0), base);
    assert!(cadence.press((0.0, 0.0), base + std::time::Duration::from_millis(50)));
    assert!(
        !cadence.press((0.0, 0.0), base + std::time::Duration::from_millis(100)),
        "the third press starts a new cadence",
    );
}

#[test]
fn geometry_off_every_monitor_is_unreachable() {
    let laptop = (0.0, 0.0, 1920.0, 1040.0);

    let on = WindowGeometry {
        position: (100.0, 100.0),
        size: (800.0, 600.0),
        maximized: false,
    };
    assert!(on.is_reachable_on(&[laptop]));

    // The classic: last seen on a second monitor that is now unplugged.
    let orphaned = WindowGeometry {
        position: (2600.0, 300.0),
        size: (800.0, 600.0),
        maximized: false,
    };
    assert!(!orphaned.is_reachable_on(&[laptop]));
    assert!(
        orphaned.is_reachable_on(&[laptop, (1920.0, 0.0, 1920.0, 1040.0)]),
        "plug the monitor back in and it is fine",
    );

    // Dragged almost entirely off the bottom: the title bar is gone, so
    // there is nothing left to grab.
    let sunk = WindowGeometry {
        position: (100.0, 1039.0),
        size: (800.0, 600.0),
        maximized: false,
    };
    assert!(!sunk.is_reachable_on(&[laptop]));

    let invalid = WindowGeometry {
        position: (100.0, 100.0),
        size: (800.0, -1.0),
        maximized: false,
    };
    assert!(!invalid.is_reachable_on(&[laptop]));
}

#[test]
fn maximized_window_keeps_its_floating_restore_rectangle() {
    let floating = WindowGeometry {
        position: (120.0, 80.0),
        size: (900.0, 640.0),
        maximized: false,
    };
    let maximized = WindowGeometry {
        position: (0.0, 0.0),
        size: (1920.0, 1040.0),
        maximized: true,
    };
    assert_eq!(
        remember_geometry(Some(floating), maximized),
        WindowGeometry {
            maximized: true,
            ..floating
        }
    );
}
