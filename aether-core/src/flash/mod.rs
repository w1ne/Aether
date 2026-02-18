//! Flash management module.
//!
//! Handles flash erase, program, and verify operations.

use anyhow::{Context, Result};
pub use probe_rs::flashing::ProgressEvent;
use probe_rs::flashing::{BinOptions, DownloadOptions, FlashProgress};
use probe_rs::Session;
use std::path::Path;
use std::sync::mpsc;

/// Progress information for flash operations.
#[derive(Debug, Clone)]
pub enum FlashingProgress {
    Started,
    EnablingDebugMode,
    Erasing,
    Programming { total: u64 },
    Progress { bytes: u32 },
    Finished,
    Failed,
    Message(String),
}

/// A progress reporter that sends updates over a channel.
pub struct MpscFlashProgress {
    sender: mpsc::Sender<FlashingProgress>,
}

impl MpscFlashProgress {
    pub fn new(sender: mpsc::Sender<FlashingProgress>) -> Self {
        Self { sender }
    }

    /// Convert this to a probe_rs::flashing::FlashProgress.
    pub fn into_flash_progress(self) -> FlashProgress<'static> {
        FlashProgress::new(move |event| {
            let update = match event {
                ProgressEvent::Started(_) => FlashingProgress::Started,
                ProgressEvent::Progress { size, .. } => {
                    FlashingProgress::Progress { bytes: size as u32 }
                }
                ProgressEvent::Finished(_) => FlashingProgress::Finished,
                ProgressEvent::Failed(_) => FlashingProgress::Failed,
                ProgressEvent::DiagnosticMessage { message } => FlashingProgress::Message(message),
                _ => return,
            };
            let _ = self.sender.send(update);
        })
    }
}

/// Manager for flash operations.
pub struct FlashManager;

impl FlashManager {
    pub fn new() -> Self {
        Self
    }

    /// Flash an ELF file to the target.
    pub fn flash_elf(
        &self,
        session: &mut Session,
        path: &Path,
        progress: FlashProgress,
    ) -> Result<()> {
        let mut options = DownloadOptions::default();
        options.progress = progress;
        options.keep_unwritten_bytes = true;

        probe_rs::flashing::download_file_with_options(
            session,
            path,
            probe_rs::flashing::Format::Elf(Default::default()),
            options,
        )
        .context("Failed to flash ELF file")
    }

    /// Flash a raw binary at a specific address.
    pub fn flash_bin(
        &self,
        session: &mut Session,
        path: &Path,
        address: u64,
        progress: FlashProgress,
    ) -> Result<()> {
        let mut options = DownloadOptions::default();
        options.progress = progress;

        let bin_options = BinOptions { base_address: Some(address), skip: 0 };

        probe_rs::flashing::download_file_with_options(
            session,
            path,
            probe_rs::flashing::Format::Bin(bin_options),
            options,
        )
        .context("Failed to flash binary file")
    }
}

impl Default for FlashManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flash_progress_enum() {
        let p = FlashingProgress::Started;
        match p {
            FlashingProgress::Started => (),
            _ => panic!("Expected Started"),
        }
    }

    #[test]
    fn test_mpsc_progress_reporting() {
        let (tx, rx) = mpsc::channel();
        let _progress = MpscFlashProgress::new(tx).into_flash_progress();

        // We cannot emit events directly as `emit` is private in probe-rs::FlashProgress.
        // This test ensures MpscFlashProgress can be created and converted without panic.
        assert!(rx.try_recv().is_err());
    }
}
