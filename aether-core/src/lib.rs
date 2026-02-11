//! Aether Core - The heart of the debugger.
//!
//! This crate handles the interaction with debug probes, target memory/registers,
//! and provides the high-performance backend for the Aether debugger.

pub mod flash;
pub mod probe;

// Re-export commonly used types
pub use flash::{FlashManager, FlashingProgress, MpscFlashProgress};
pub use probe::{ProbeInfo, ProbeManager, ProbeType, TargetInfo};
