//! Aether Core - The heart of the debugger.
//!
//! This crate handles the interaction with debug probes, target memory/registers,
//! and provides the high-performance backend for the Aether debugger.

pub mod debug;
pub mod disasm;
pub mod flash;
pub mod memory;
pub mod probe;
pub mod session;
pub mod svd;

// Re-export commonly used types
pub use debug::DebugManager;
pub use disasm::DisassemblyManager;
pub use flash::{FlashManager, FlashingProgress, MpscFlashProgress};
pub use memory::MemoryManager;
pub use probe::{ProbeInfo, ProbeManager, ProbeType, TargetInfo};
pub use svd::SvdManager;
pub use session::{DebugCommand, DebugEvent, SessionHandle};
