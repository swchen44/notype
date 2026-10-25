#[path = "../../desktop/src-tauri/src/window_lifecycle.rs"]
mod lifecycle;

use lifecycle::{close_to_tray, MainWindow};
use std::cell::RefCell;

struct Desktop {
    tray_available: bool,
    operations: RefCell<Vec<&'static str>>,
}

impl MainWindow for Desktop {
    fn ensure_tray_visible(&self) -> Result<(), String> {
        self.operations.borrow_mut().push("ensure-tray");
        self.tray_available
            .then_some(())
            .ok_or("tray-unavailable".into())
    }
    fn hide(&self) -> Result<(), String> {
        self.operations.borrow_mut().push("hide-window");
        Ok(())
    }
}

#[test]
fn close_keeps_window_visible_when_native_tray_cannot_be_restored() {
    let desktop = Desktop {
        tray_available: false,
        operations: RefCell::default(),
    };
    assert_eq!(close_to_tray(&desktop), Err("tray-unavailable".into()));
    assert_eq!(*desktop.operations.borrow(), vec!["ensure-tray"]);
}

#[test]
fn close_hides_only_after_the_native_tray_is_confirmed() {
    let desktop = Desktop {
        tray_available: true,
        operations: RefCell::default(),
    };
    assert_eq!(close_to_tray(&desktop), Ok(()));
    assert_eq!(
        *desktop.operations.borrow(),
        vec!["ensure-tray", "hide-window"]
    );
}
