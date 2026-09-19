//! Notifier implementations that do not depend on a desktop shell.

use ubiqx_core::ports::Notifier;
use ubiqx_core::CoreResult;

/// Logs notifications through `tracing`. Used by the CLI and tests; the desktop app installs
/// a real notifier backed by the OS notification centre.
#[derive(Debug, Default, Clone, Copy)]
pub struct LogNotifier;

impl Notifier for LogNotifier {
    fn notify(&self, title: &str, body: &str) -> CoreResult<()> {
        tracing::info!(target: "ubiqx::notify", %title, %body, "notification");
        Ok(())
    }
}
