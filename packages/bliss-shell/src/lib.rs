#![cfg_attr(docsrs, feature(doc_cfg))]

//! Event loop, windowing and system integration.
//!
//! ## Feature flags
//!  - `default`: Enables the features listed below.
//!  - `accessibility`: Enables [`accesskit`] accessibility support.
//!  - `hot-reload`: Enables hot-reloading of Dioxus RSX.
//!  - `tracing`: Enables tracing support.
//!
//! Platform support: Linux + Android. macOS / iOS / Windows / *BSD `cfg`
//! branches inside source remain as future-port anchors (see workspace-meta
//! `FOREMAN_THREADS.md` POSIX-scope thread) but the crate as a whole is
//! gated to Linux/Android via `compile_error!` below.

#[cfg(not(any(target_os = "linux", target_os = "android")))]
compile_error!(
    "bliss-shell is Linux/Android-only by policy. \
    See projects/exosphere/CLAUDE.md \"Platform support\"."
);

mod application;
mod convert_events;
mod event;
mod net;
mod window;

#[cfg(feature = "exoshell")]
pub mod exoshell;

#[cfg(feature = "accessibility")]
mod accessibility;

pub use crate::application::BlissApplication;
pub use crate::event::{BlissShellEvent, BlissShellProxy};
pub use crate::window::{View, WindowConfig};

#[cfg(feature = "data-uri")]
pub use crate::net::DataUriNetProvider;

#[cfg(all(
    feature = "file_dialog",
    any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    )
))]
use bliss_traits::shell::FileDialogFilter;
use bliss_traits::shell::ShellProvider;
use std::sync::Arc;
use winit::cursor::{Cursor, CursorIcon};
use winit::dpi::{LogicalPosition, LogicalSize};
pub use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
pub use winit::window::Window;
use winit::window::{ImeCapabilities, ImeEnableRequest, ImeRequest, ImeRequestData};

#[derive(Default)]
pub struct Config {
    pub stylesheets: Vec<String>,
    pub base_url: Option<String>,
}

/// Build an event loop for the application
pub fn create_default_event_loop() -> EventLoop {
    let mut ev_builder = EventLoop::builder();
    #[cfg(target_os = "android")]
    {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        ev_builder.with_android_app(current_android_app());
    }

    let event_loop = ev_builder.build().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    event_loop
}

#[cfg(target_os = "android")]
static ANDROID_APP: std::sync::OnceLock<android_activity::AndroidApp> = std::sync::OnceLock::new();

#[cfg(target_os = "android")]
#[cfg_attr(docsrs, doc(cfg(target_os = "android")))]
/// Set the current [`AndroidApp`](android_activity::AndroidApp).
pub fn set_android_app(app: android_activity::AndroidApp) {
    ANDROID_APP.set(app).unwrap()
}

#[cfg(target_os = "android")]
#[cfg_attr(docsrs, doc(cfg(target_os = "android")))]
/// Get the current [`AndroidApp`](android_activity::AndroidApp).
/// This will panic if the android activity has not been setup with [`set_android_app`].
pub fn current_android_app() -> android_activity::AndroidApp {
    ANDROID_APP.get().unwrap().clone()
}

pub struct BlissShellProvider {
    window: Arc<dyn Window>,
}
impl BlissShellProvider {
    pub fn new(window: Arc<dyn Window>) -> Self {
        Self { window }
    }
}

impl ShellProvider for BlissShellProvider {
    fn request_redraw(&self) {
        self.window.request_redraw();
    }
    fn set_cursor(&self, icon: CursorIcon) {
        self.window.set_cursor(Cursor::Icon(icon));
    }
    fn set_window_title(&self, title: String) {
        self.window.set_title(&title);
    }
    fn set_ime_enabled(&self, is_enabled: bool) {
        if is_enabled {
            let _ = self.window.request_ime_update(ImeRequest::Enable(
                ImeEnableRequest::new(ImeCapabilities::new(), ImeRequestData::default()).unwrap(),
            ));
        } else {
            let _ = self.window.request_ime_update(ImeRequest::Disable);
        }
    }
    fn set_ime_cursor_area(&self, x: f32, y: f32, width: f32, height: f32) {
        let _ = self.window.request_ime_update(ImeRequest::Update(
            ImeRequestData::default().with_cursor_area(
                LogicalPosition::new(x, y).into(),
                LogicalSize::new(width, height).into(),
            ),
        ));
    }

    #[cfg(all(
        feature = "clipboard",
        any(
            target_os = "windows",
            target_os = "macos",
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        )
    ))]
    fn get_clipboard_text(&self) -> Result<String, bliss_traits::shell::ClipboardError> {
        let mut cb = arboard::Clipboard::new().unwrap();
        cb.get_text()
            .map_err(|_| bliss_traits::shell::ClipboardError)
    }

    #[cfg(all(
        feature = "clipboard",
        any(
            target_os = "windows",
            target_os = "macos",
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        )
    ))]
    fn set_clipboard_text(&self, text: String) -> Result<(), bliss_traits::shell::ClipboardError> {
        let mut cb = arboard::Clipboard::new().unwrap();
        cb.set_text(text.to_owned())
            .map_err(|_| bliss_traits::shell::ClipboardError)
    }

    #[cfg(all(
        feature = "file_dialog",
        any(
            target_os = "windows",
            target_os = "macos",
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        )
    ))]
    fn open_file_dialog(
        &self,
        multiple: bool,
        filter: Option<FileDialogFilter>,
    ) -> Vec<std::path::PathBuf> {
        let mut dialog = rfd::FileDialog::new();
        if let Some(FileDialogFilter { name, extensions }) = filter {
            dialog = dialog.add_filter(&name, &extensions);
        }
        let files = if multiple {
            dialog.pick_files()
        } else {
            dialog.pick_file().map(|file| vec![file])
        };
        files.unwrap_or_default()
    }

    /// Show a platform-native context menu at the given screen coordinates.
    ///
    /// winit does not provide a native context menu API, so this requires
    /// platform-specific shell integration. The context menu event is still
    /// properly dispatched to the script engine — scripts can handle it via
    /// JavaScript event listeners and render custom in-app menus.
    ///
    /// Platform-specific implementation notes:
    /// - Linux: Requires a GUI toolkit (GTK popover via gtk-rs) or
    ///   xdg-desktop-portal menu (ashpd/zbus)
    /// - macOS: Uses NSMenu via objc2
    /// - Windows: Uses TrackPopupMenu via winapi
    fn show_context_menu(&self, x: f64, y: f64) {
        let _ = (x, y);
        // No native context menu is available without platform-specific
        // GUI toolkit dependencies. The event system handles the rest.
    }
}
