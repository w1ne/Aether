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
pub use flash::{FlashManager, FlashingProgress, MpscFlashProgress};
pub use memory::MemoryManager;
pub use probe_rs::{CoreStatus, RegisterValue};

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

    pub trait MemoryInterface {
        fn read(&mut self, address: u64, data: &mut [u8]) -> anyhow::Result<()> {
            let _ = (address, data);
            anyhow::bail!("Hardware support disabled")
        }
        fn read_8(&mut self, address: u64, data: &mut [u8]) -> anyhow::Result<()> {
            self.read(address, data)
        }
        fn read_word_32(&mut self, _address: u64) -> anyhow::Result<u32> {
            anyhow::bail!("Hardware support disabled")
        }
        fn read_word_16(&mut self, _address: u64) -> anyhow::Result<u16> {
            anyhow::bail!("Hardware support disabled")
        }
        fn read_word_8(&mut self, _address: u64) -> anyhow::Result<u8> {
            anyhow::bail!("Hardware support disabled")
        }
        fn write_8(&mut self, address: u64, data: &[u8]) -> anyhow::Result<()> {
            let _ = (address, data);
            anyhow::bail!("Hardware support disabled")
        }
        fn write_word_32(&mut self, address: u64, data: u32) -> anyhow::Result<()> {
            self.write_8(address, &data.to_le_bytes())
        }
        fn write_word_16(&mut self, address: u64, data: u16) -> anyhow::Result<()> {
            self.write_8(address, &data.to_le_bytes())
        }
        fn write_word_8(&mut self, address: u64, data: u8) -> anyhow::Result<()> {
            self.write_8(address, &[data])
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TraceSink {
        TraceMemory,
        Swo,
        Tpiu,
    }

    #[derive(Clone)]
    pub struct Session;
    impl Session {
        pub fn target(&self) -> Target {
            Target
        }
        pub fn core(&mut self, _i: usize) -> anyhow::Result<Core> {
            anyhow::bail!("Hardware support disabled")
        }
        pub fn setup_tracing(&mut self, _id: usize, _sink: TraceSink) -> anyhow::Result<()> {
            anyhow::bail!("Hardware support disabled")
        }
        pub fn read_trace_data(&mut self) -> anyhow::Result<Vec<u8>> {
            Ok(vec![])
        }
    }

    pub enum ProgressEvent {
        Started(u64),
        Progress { size: u64, current: u64 },
        Finished(u64),
        Failed(u64),
        DiagnosticMessage { message: String },
    }

    pub struct FlashProgress;
    impl FlashProgress {
        pub fn new(_f: impl Fn(ProgressEvent) + Send + 'static) -> Self {
            Self
        }
    }

    pub struct Core;
    impl MemoryInterface for Core {}
    impl Core {
        pub fn read_core_reg(&mut self, _addr: u32) -> anyhow::Result<RegisterValue> {
            anyhow::bail!("Hardware support disabled")
        }
        pub fn write_core_reg(&mut self, _addr: u32, _val: RegisterValue) -> anyhow::Result<()> {
            anyhow::bail!("Hardware support disabled")
        }
        pub fn program_counter(&self) -> u32 {
            0
        }
        pub fn stack_pointer(&self) -> u32 {
            0
        }
        pub fn return_address(&self) -> u32 {
            0
        }
        pub fn run(&mut self) -> anyhow::Result<()> {
            anyhow::bail!("Hardware support disabled")
        }
        pub fn info(&self) -> anyhow::Result<CoreInformation> {
            anyhow::bail!("Hardware support disabled")
        }
        pub fn set_hw_breakpoint(&mut self, _addr: u64) -> anyhow::Result<()> {
            anyhow::bail!("Hardware support disabled")
        }
        pub fn clear_hw_breakpoint(&mut self, _addr: u64) -> anyhow::Result<()> {
            anyhow::bail!("Hardware support disabled")
        }
        pub fn halt(&mut self, _timeout: std::time::Duration) -> anyhow::Result<CoreInformation> {
            anyhow::bail!("Hardware support disabled")
        }
        pub fn step(&mut self) -> anyhow::Result<CoreInformation> {
            anyhow::bail!("Hardware support disabled")
        }
        pub fn status(&mut self) -> anyhow::Result<CoreStatus> {
            anyhow::bail!("Hardware support disabled")
        }
    }

    pub struct CoreInformation {
        pub pc: u32,
    }

    pub struct Target;
    impl Target {
        pub fn architecture(&self) -> Architecture {
            Architecture::Arm
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Architecture {
        Arm,
        Riscv,
    }

    pub enum RegisterValue {
        U32(u32),
        U64(u64),
        U128(u128),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum CoreStatus {
        Running,
        Halted(HaltReason),
        Unknown,
        LockedUp,
        Sleeping,
    }
    impl CoreStatus {
        pub fn is_halted(&self) -> bool {
            matches!(self, CoreStatus::Halted(_))
        }
    }
}

#[cfg(not(feature = "hardware"))]
pub mod probe_rs_debug {
    pub struct DebugInfo;
    impl DebugInfo {
        pub fn from_file(_path: &std::path::Path) -> anyhow::Result<Self> {
            anyhow::bail!("Hardware support disabled")
        }
        pub fn get_source_location(&self, _address: u64) -> Option<SourceLocation> {
            None
        }
    }
    pub struct SourceLocation {
        pub path: std::path::PathBuf,
        pub line: Option<u64>,
        pub column: Option<ColumnType>,
    }
    pub enum ColumnType {
        Column(u64),
        LeftEdge,
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
#[cfg(not(feature = "hardware"))]
pub mod probe {
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ProbeInfo {
        pub vendor_id: u16,
        pub product_id: u16,
        pub serial_number: Option<String>,
        pub identifier: String,
        pub probe_type: ProbeType,
    }
    impl ProbeInfo {
        pub fn name(&self) -> String {
            self.identifier.clone()
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
    pub enum ProbeType {
        JLink,
        STLink,
        CmsisDap,
        Other,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct TargetInfo {
        pub name: String,
        pub flash_size: u64,
        pub ram_size: u64,
        pub architecture: String,
    }

    #[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
    pub enum WireProtocol {
        Swd,
        Jtag,
    }

    pub struct ProbeManager;
    impl ProbeManager {
        pub fn new() -> Self {
            Self
        }
        pub fn list_probes(&self) -> anyhow::Result<Vec<ProbeInfo>> {
            Ok(vec![])
        }
        pub fn connect(
            &self,
            _idx: usize,
            _chip: &str,
            _proto: Option<WireProtocol>,
            _reset: bool,
        ) -> anyhow::Result<(TargetInfo, crate::probe_rs::Session)> {
            anyhow::bail!("Hardware support disabled")
        }
    }
    pub fn map_probe_error(e: &anyhow::Error) -> String {
        e.to_string()
    }
}

#[cfg(not(feature = "hardware"))]
pub use probe::{ProbeInfo, ProbeManager, ProbeType, TargetInfo, WireProtocol};
#[cfg(feature = "hardware")]
pub use probe::{ProbeInfo, ProbeManager, ProbeType, TargetInfo, WireProtocol};
pub use session::{DebugCommand, DebugEvent, SessionHandle};
pub use stack::StackFrame;
pub use svd::SvdManager;
pub use symbols::{SourceInfo, SymbolManager};

#[cfg(not(feature = "hardware"))]
pub mod svd {
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct PeripheralInfo {
        pub name: String,
        pub base_address: u64,
        pub description: Option<String>,
        pub registers: Vec<RegisterInfo>,
    }
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct RegisterInfo {
        pub name: String,
        pub address_offset: u32,
        pub description: Option<String>,
        pub size: u32,
        pub fields: Vec<FieldInfo>,
        pub value: Option<u64>,
    }
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct FieldInfo {
        pub name: String,
        pub bit_offset: u32,
        pub bit_width: u32,
        pub value: u64,
        pub description: Option<String>,
    }
    impl FieldInfo {
        pub fn decode(&self, _val: u64) -> u64 {
            0
        }
    }
    pub struct SvdManager;
    impl SvdManager {
        pub fn new() -> Self {
            Self
        }
        pub fn get_peripherals_info(&self) -> Vec<PeripheralInfo> {
            vec![]
        }
    }
}

#[cfg(not(feature = "hardware"))]
pub mod disasm {
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct InstructionInfo {
        pub address: u64,
        pub mnemonic: String,
        pub op_str: String,
        pub bytes: Vec<u8>,
    }
    pub struct DisassemblyManager;
    impl DisassemblyManager {
        pub fn new() -> Self {
            Self
        }
    }
}

#[cfg(not(feature = "hardware"))]
pub mod flash {
    pub struct FlashManager;
    impl FlashManager {
        pub fn new() -> Self {
            Self
        }
    }
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
    pub struct MpscFlashProgress {
        _sender: std::sync::mpsc::Sender<FlashingProgress>,
    }
    impl MpscFlashProgress {
        pub fn new(sender: std::sync::mpsc::Sender<FlashingProgress>) -> Self {
            Self { _sender: sender }
        }
        pub fn into_flash_progress(self) -> crate::probe_rs::FlashProgress {
            crate::probe_rs::FlashProgress
        }
    }
}
