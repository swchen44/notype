//! Platform-independent close-to-tray contract. Explicit Quit remains an app exit.

/// Native operations required before the main window may leave the screen.
pub trait MainWindow {
    /// Restore or create the native tray and confirm its visibility operation.
    fn ensure_tray_visible(&self) -> Result<(), String>;
    /// Hide the main window after a return path has been confirmed.
    fn hide(&self) -> Result<(), String>;
}

/// A close request keeps the app running and requires a usable return path.
pub fn close_to_tray(window: &impl MainWindow) -> Result<(), String> {
    window.ensure_tray_visible()?;
    window.hide()
}
