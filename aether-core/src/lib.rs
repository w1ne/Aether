//! Aether Core - The heart of the debugger.
//!
//! This crate handles the interaction with debug probes, target memory/registers,
//! and provides the high-performance backend for the Aether debugger.

pub mod debug;
pub mod disasm;
pub mod flash;
pub mod memory;
pub mod probe;
pub mod rtt;
pub mod rtos;
pub mod session;
pub mod svd;
pub mod symbols;
pub mod semihosting;
pub mod itm;

pub mod stack;
pub mod trace;


// Re-export commonly used types
pub use debug::DebugManager;
pub use disasm::DisassemblyManager;
pub use flash::{FlashManager, FlashingProgress, MpscFlashProgress};
pub use memory::MemoryManager;
pub use probe_rs::CoreStatus;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum VarType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskInfo {
    pub name: String,
    pub priority: u32,
    pub state: TaskState,
    pub stack_usage: u32,
    pub stack_size: u32,
    pub handle: u32, // address of TCB or Task handle
    pub task_type: TaskType,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TaskType {
    Thread,
    Async,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TaskState {
    Running,
    Ready,
    Blocked,
    Suspended,
    Deleted,
    Pending,
    Unknown,
}
pub use probe::{ProbeInfo, ProbeManager, ProbeType, TargetInfo, WireProtocol};
pub use svd::SvdManager;
pub use symbols::{SymbolManager, SourceInfo};
pub use session::{DebugCommand, DebugEvent, SessionHandle};
pub use stack::StackFrame;
