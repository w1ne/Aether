//! Aether Core - The heart of the debugger.
//!
//! This crate handles the interaction with debug probes, target memory/registers,
//! and provides the high-performance backend for the Aether debugger.

pub mod debug;
#[cfg(feature = "hardware")]
pub mod disasm;
#[cfg(feature = "hardware")]
pub mod flash;
pub mod itm;
pub mod memory;
#[cfg(feature = "hardware")]
pub mod probe;
pub mod rtos;
pub mod rtt;
pub mod semihosting;
pub mod session;
#[cfg(feature = "hardware")]
pub mod svd;
pub mod symbols;

pub mod stack;
pub mod trace;

// Re-export commonly used types
pub use debug::DebugManager;
#[cfg(feature = "hardware")]
pub use disasm::DisassemblyManager;
#[cfg(feature = "hardware")]
pub use flash::{FlashManager, FlashingProgress, MpscFlashProgress};
pub use memory::MemoryManager;
#[cfg(feature = "hardware")]
pub use probe_rs::CoreStatus;

#[cfg(not(feature = "hardware"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CoreStatus {
    Running,
    Halted(probe_rs::HaltReason),
    Unknown,
    LockedUp,
    Sleeping,
}

#[cfg(not(feature = "hardware"))]
pub mod probe_rs {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum HaltReason {
        Breakpoint,
        Step,
        External,
        Request,
        Exception,
        Other,
    }

    #[derive(Clone)]
    pub struct Session;
    impl Session {
        pub fn target(&self) -> Target { Target }
    }

    pub struct Target;
    impl Target {
        pub fn architecture(&self) -> Architecture { Architecture }
    }

    pub enum Architecture {
        Arm,
        Riscv,
    }
}

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
#[cfg(feature = "hardware")]
pub use probe::{ProbeInfo, ProbeManager, ProbeType, TargetInfo, WireProtocol};
pub use session::{DebugCommand, DebugEvent, SessionHandle};
pub use stack::StackFrame;
#[cfg(feature = "hardware")]
pub use svd::SvdManager;
pub use symbols::{SourceInfo, SymbolManager};
