//! Optional ExoShell integration for bliss-shell.
//!
//! When the `exoshell` feature is enabled, BlissApplication can register
//! with the running ExoShell daemon for session management, capability gating,
//! and cross-shell event routing.
//!
//! If ExoShell is not running, registration silently fails and the application
//! continues standalone.

use exo_shell_client::{Registration, ShellType, try_register};

/// ExoShell connection state for a bliss-shell application
pub enum ExoShellState {
    /// Not connected — ExoShell unavailable or feature disabled
    Standalone,
    /// Registered with ExoShell
    Connected(Registration),
}

impl ExoShellState {
    /// Attempt to register this GUI shell with ExoShell.
    ///
    /// Returns `Connected` if successful, `Standalone` if ExoShell is unavailable.
    pub async fn try_connect(name: &str, capabilities: Vec<String>) -> Self {
        match try_register(name, ShellType::BlissGui, capabilities).await {
            Some(reg) => {
                tracing::info!(
                    name,
                    session = %reg.session_id.0,
                    shell = %reg.shell_id.0,
                    "Connected to ExoShell"
                );
                Self::Connected(reg)
            }
            None => {
                tracing::debug!("ExoShell not available, running standalone");
                Self::Standalone
            }
        }
    }

    /// Check if connected to ExoShell
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected(_))
    }

    /// Cleanly disconnect on shutdown
    pub async fn disconnect(self) {
        if let Self::Connected(reg) = self {
            reg.unregister().await;
        }
    }
}
