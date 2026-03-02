//! Session management module.
//!
//! Handles the debug session in a background thread, processing commands
//! and sending events back to the main thread.

use crate::debug::DebugManager;
use crate::CoreStatus;
use crate::VarType;
use anyhow::{Context as _, Result};
use crossbeam_channel::{Receiver, Sender};
#[cfg(feature = "hardware")]
use probe_rs::flashing::{FlashProgress, ProgressEvent};
#[cfg(feature = "hardware")]
use probe_rs::{MemoryInterface, Session};
#[cfg(feature = "hardware")]
use probe_rs_debug::SteppingMode;
#[cfg(feature = "hardware")]
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
#[cfg(feature = "hardware")]
use std::time::Instant;

#[derive(Debug)]
pub enum DebugCommand {
    Halt,
    Resume,
    Step,
    StepOver,
    StepInto,
    StepOut,
    ReadRegister(u16),
    WriteRegister(u16, u64),
    ReadMemory(u64, usize),
    WriteMemory(u64, Vec<u8>),
    Disassemble(u64, usize),
    SetBreakpoint(u64),
    ClearBreakpoint(u64),
    ListBreakpoints,
    LoadSvd(std::path::PathBuf),
    LoadSymbols(std::path::PathBuf),
    LookupSource(u64),
    ToggleBreakpointAtSource(std::path::PathBuf, u32),
    GetPeripherals,
    GetRegisters(String),
    ReadPeripheralValues(String),
    WritePeripheralField {
        peripheral: String,
        register: String,
        field: String,
        value: u64,
    },
    RttAttach,
    RttWrite {
        channel: usize,
        data: Vec<u8>,
    },
    PollStatus,
    AddPlot {
        name: String,
        var_type: VarType,
    },
    RemovePlot(String),
    WatchVariable(String),
    GetTasks,
    GetStack,
    EnableTrace(crate::trace::TraceConfig),
    Exit,
    StartFlashing(std::path::PathBuf),
    EnableSemihosting,
    EnableItm {
        baud_rate: u32,
    },
    ListProbes,
    Attach {
        probe_index: usize,
        chip: String,
        protocol: Option<crate::probe::WireProtocol>,
        under_reset: bool,
    },
    Reset,
    AttachSubSession {
        name: String,
        probe_index: usize,
        chip: String,
        protocol: Option<crate::probe::WireProtocol>,
        under_reset: bool,
    },
    SetActiveTarget(String),
    ShadowSync {
        master: String,
        slave: String,
    },
    ShadowStep,
}

struct PlotConfig {
    name: String,
    address: u64,
    var_type: VarType,
}

#[derive(Debug, Clone)]
pub enum DebugEvent {
    Halted {
        pc: u64,
    },
    Resumed,
    RegisterValue(u16, u64),
    MemoryData(u64, Vec<u8>),
    Disassembly(Vec<crate::disasm::InstructionInfo>),
    Breakpoints(Vec<u64>),
    SvdLoaded,
    Peripherals(Vec<crate::svd::PeripheralInfo>),
    Registers(Vec<crate::svd::RegisterInfo>),
    SymbolsLoaded,
    SourceLocation(crate::symbols::SourceInfo),
    BreakpointLocations(Vec<crate::symbols::SourceInfo>),
    RttChannels {
        up_channels: Vec<crate::rtt::RttChannelInfo>,
        down_channels: Vec<crate::rtt::RttChannelInfo>,
    },
    RttData(usize, Vec<u8>),
    PlotData {
        name: String,
        timestamp: f64,
        value: f64,
    },
    #[cfg(feature = "hardware")]
    Tasks(Vec<crate::TaskInfo>),
    #[cfg(not(feature = "hardware"))]
    Tasks(Vec<crate::TaskInfo>),
    TaskSwitch {
        from: Option<u32>,
        to: u32,
        timestamp: f64,
    },
    #[cfg(feature = "hardware")]
    Stack(Vec<crate::stack::StackFrame>),
    #[cfg(not(feature = "hardware"))]
    Stack(Vec<crate::stack::StackFrame>),
    TraceData(Vec<u8>),
    Status(CoreStatus),
    Error(String),
    FlashProgress(f32),
    FlashStatus(String),
    FlashDone,
    VariableResolved(crate::symbols::TypeInfo),
    SemihostingOutput(String),
    ItmPacket(Vec<u8>),
    #[cfg(feature = "hardware")]
    Probes(Vec<crate::probe::ProbeInfo>),
    #[cfg(not(feature = "hardware"))]
    Probes(Vec<crate::probe::ProbeInfo>),
    #[cfg(feature = "hardware")]
    Attached(crate::probe::TargetInfo),
    #[cfg(not(feature = "hardware"))]
    Attached(crate::probe::TargetInfo),
    #[cfg(feature = "hardware")]
    SubSessionAttached(String, crate::probe::TargetInfo),
    #[cfg(not(feature = "hardware"))]
    SubSessionAttached(String, crate::probe::TargetInfo),
    ParityDiverged {
        location: u64,
        master_val: u64,
        slave_val: u64,
        info: String,
    },
}

/// A handle to the debug session running in a background thread.
pub struct SessionHandle {
    command_tx: Sender<DebugCommand>,
    event_tx: tokio::sync::broadcast::Sender<DebugEvent>,
    #[allow(dead_code)] // Kept for future graceful shutdown
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl SessionHandle {
    /// Subscribe to debug events
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<DebugEvent> {
        self.event_tx.subscribe()
    }

    /// Internal helper to create a SessionHandle for testing
    pub fn new_test() -> (Self, Receiver<DebugCommand>, tokio::sync::broadcast::Sender<DebugEvent>)
    {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let (evt_tx, _) = tokio::sync::broadcast::channel(1024);

        (Self { command_tx: cmd_tx, event_tx: evt_tx.clone(), thread_handle: None }, cmd_rx, evt_tx)
    }

    #[cfg(feature = "hardware")]
    pub fn new(session: Option<Session>) -> Result<Self> {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        // create a broadcast channel with capacity 100
        let (evt_tx, _) = tokio::sync::broadcast::channel(100);
        let evt_tx_thread = evt_tx.clone();

        let thread_handle = thread::spawn(move || {
            let mut sessions: HashMap<String, Session> = HashMap::new();
            let mut active_target = "default".to_string();
            if let Some(s) = session {
                sessions.insert(active_target.clone(), s);
            }
            let mut shadow_sync: Option<(String, String)> = None;

            let evt_tx = evt_tx_thread; // Shadow for inner scope
            let debug_manager = DebugManager::new();
            let _memory_manager = crate::MemoryManager::new();
            let disasm_manager = crate::disasm::DisassemblyManager::new();
            let mut breakpoint_manager = crate::debug::BreakpointManager::new();
            let mut svd_manager = crate::svd::SvdManager::new();
            let mut rtt_manager = crate::rtt::RttManager::new();
            let mut symbol_manager = crate::symbols::SymbolManager::new();
            let mut trace_manager = crate::trace::TraceManager::new();
            let mut rtos_manager: Option<Box<dyn crate::rtos::RtosAware>> = None;
            let mut _last_poll = Instant::now();
            let mut core_status = None;
            let mut itm_manager = crate::itm::ItmManager::new();

            let mut plots: Vec<PlotConfig> = Vec::new();
            let mut last_plot_poll = Instant::now();
            let mut _last_task_handle: Option<u32> = None;
            let mut _last_status_poll = Instant::now();

            let mut arch =
                sessions.get(&active_target).map(|s| format!("{:?}", s.target().architecture()));
            let session_start = Instant::now();

            // Loop for processing commands and events
            loop {
                // 1. Trace Polling (needs &mut Session)
                for s in sessions.values_mut() {
                    if let Ok(data) = trace_manager.read_data(s) {
                        if !data.is_empty() {
                            let _ = evt_tx.send(DebugEvent::TraceData(data));
                        }
                    }
                }

                // 2. Commands (Session or Core)
                let cmd_opt = cmd_rx.try_recv().ok();

                if let Some(cmd) = cmd_opt {
                    #[allow(unreachable_patterns)]
                    match cmd {
                        DebugCommand::EnableTrace(config) => {
                            if let Some(s) = sessions.get_mut(&active_target) {
                                if let Err(e) = trace_manager.enable(s, config) {
                                    let _ = evt_tx.send(DebugEvent::Error(format!(
                                        "Failed to enable trace: {}",
                                        e
                                    )));
                                }
                            } else {
                                let _ = evt_tx.send(DebugEvent::Error(format!(
                                    "No active session for {}",
                                    active_target
                                )));
                            }
                            continue;
                        }
                        DebugCommand::Exit => return,
                        DebugCommand::StartFlashing(path) => {
                            if let Some(s) = sessions.get_mut(&active_target) {
                                let flash_manager = crate::flash::FlashManager::new();
                                let tx_clone = evt_tx.clone();
                                let total_size =
                                    std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
                                let current_size =
                                    std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
                                let progress = FlashProgress::new(move |event| {
                                    let size_ref = total_size.clone();
                                    let current_ref = current_size.clone();
                                    let update = match event {
                                        ProgressEvent::AddProgressBar { total, .. } => {
                                            if let Some(t) = total {
                                                size_ref.fetch_add(
                                                    t,
                                                    std::sync::atomic::Ordering::Relaxed,
                                                );
                                            }
                                            return;
                                        }
                                        ProgressEvent::Started(op) => {
                                            DebugEvent::FlashStatus(format!("{:?}", op))
                                        }
                                        ProgressEvent::Progress { size, .. } => {
                                            let total =
                                                size_ref.load(std::sync::atomic::Ordering::Relaxed);
                                            let current = current_ref.fetch_add(
                                                size,
                                                std::sync::atomic::Ordering::Relaxed,
                                            ) + size;
                                            if total > 0 {
                                                DebugEvent::FlashProgress(
                                                    current as f32 / total as f32,
                                                )
                                            } else {
                                                DebugEvent::FlashProgress(0.0)
                                            }
                                        }
                                        ProgressEvent::Finished(_) => {
                                            DebugEvent::FlashStatus("Finished".to_string())
                                        }
                                        ProgressEvent::Failed(_) => {
                                            DebugEvent::Error("Flash failed".to_string())
                                        }
                                        _ => return,
                                    };
                                    let _ = tx_clone.send(update);
                                });
                                // Note: We use the session directly here as before
                                match flash_manager.flash_elf(s, &path, progress) {
                                    Ok(_) => {
                                        let _ = evt_tx.send(DebugEvent::FlashDone);
                                    }
                                    Err(e) => {
                                        let _ = evt_tx.send(DebugEvent::Error(format!(
                                            "Flash failed: {}",
                                            e
                                        )));
                                    }
                                }
                            } else {
                                let _ = evt_tx.send(DebugEvent::Error(format!(
                                    "No active session for {}",
                                    active_target
                                )));
                            }
                            continue;
                        }
                        DebugCommand::EnableSemihosting => {
                            log::info!("Semihosting enabled");
                            continue;
                        }
                        DebugCommand::EnableItm { baud_rate } => {
                            if let Some(s) = sessions.get_mut(&active_target) {
                                if let Err(e) = itm_manager.configure(s, baud_rate) {
                                    let _ = evt_tx.send(DebugEvent::Error(format!(
                                        "Failed to enable ITM: {}",
                                        e
                                    )));
                                } else {
                                    log::info!("ITM enabled at {} baud", baud_rate);
                                }
                            } else {
                                let _ = evt_tx.send(DebugEvent::Error(format!(
                                    "No active session for {}",
                                    active_target
                                )));
                            }
                            continue;
                        }
                        DebugCommand::ListProbes => {
                            let pm = crate::probe::ProbeManager::new();
                            match pm.list_probes() {
                                Ok(p) => {
                                    let _ = evt_tx.send(DebugEvent::Probes(p));
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!(
                                        "Failed to list probes: {}",
                                        e
                                    )));
                                }
                            }
                            continue;
                        }
                        DebugCommand::Attach { probe_index, chip, protocol, under_reset } => {
                            let pm = crate::probe::ProbeManager::new();
                            match pm.connect(probe_index, &chip, protocol, under_reset) {
                                Ok((info, s)) => {
                                    sessions.insert(active_target.clone(), s);
                                    arch = Some(info.architecture.clone());
                                    let _ = evt_tx.send(DebugEvent::Attached(info));
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!(
                                        "Failed to attach: {}",
                                        crate::probe::map_probe_error(&e)
                                    )));
                                }
                            }
                            continue;
                        }
                        DebugCommand::AttachSubSession {
                            name,
                            probe_index,
                            chip,
                            protocol,
                            under_reset,
                        } => {
                            let pm = crate::probe::ProbeManager::new();
                            match pm.connect(probe_index, &chip, protocol, under_reset) {
                                Ok((info, s)) => {
                                    sessions.insert(name.clone(), s);
                                    let _ = evt_tx.send(DebugEvent::SubSessionAttached(name, info));
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!(
                                        "Failed to attach sub-session {}: {}",
                                        name, e
                                    )));
                                }
                            }
                            continue;
                        }
                        DebugCommand::SetActiveTarget(name) => {
                            active_target = name;
                            continue;
                        }
                        DebugCommand::ShadowSync { master, slave } => {
                            shadow_sync = Some((master, slave));
                            continue;
                        }
                        // Core commands
                        // Core commands
                        #[allow(unreachable_patterns)]
                        core_cmd => {
                            let target_names = if let Some((ref m, ref s)) = shadow_sync {
                                if matches!(
                                    core_cmd,
                                    DebugCommand::Halt
                                        | DebugCommand::Resume
                                        | DebugCommand::Step
                                        | DebugCommand::StepOver
                                        | DebugCommand::StepInto
                                        | DebugCommand::StepOut
                                        | DebugCommand::Reset
                                        | DebugCommand::ShadowStep
                                ) {
                                    vec![m.clone(), s.clone()]
                                } else {
                                    vec![active_target.clone()]
                                }
                            } else {
                                vec![active_target.clone()]
                            };

                            let mut halt_pcs = Vec::new();

                            for name in &target_names {
                                let s = match sessions.get_mut(name) {
                                    Some(s) => s,
                                    None => {
                                        let _ = evt_tx.send(DebugEvent::Error(format!(
                                            "No active session for {}",
                                            name
                                        )));
                                        continue;
                                    }
                                };
                                let mut core = match s.core(0) {
                                    Ok(c) => c,
                                    Err(e) => {
                                        let _ = evt_tx.send(DebugEvent::Error(format!(
                                            "Failed to attach core: {}",
                                            e
                                        )));
                                        continue;
                                    }
                                };

                                match &core_cmd {
                                    DebugCommand::Halt => match debug_manager.halt(&mut core) {
                                        Ok(info) => {
                                            halt_pcs.push((name.clone(), info.pc));
                                            let _ = evt_tx.send(DebugEvent::Halted { pc: info.pc });
                                        }
                                        Err(e) => {
                                            let _ = evt_tx.send(DebugEvent::Error(format!(
                                                "Failed to halt {}: {}",
                                                name, e
                                            )));
                                        }
                                    },
                                    DebugCommand::Resume => match debug_manager.resume(&mut core) {
                                        Ok(_) => {
                                            let _ = evt_tx.send(DebugEvent::Resumed);
                                        }
                                        Err(e) => {
                                            let _ = evt_tx.send(DebugEvent::Error(format!(
                                                "Failed to resume {}: {}",
                                                name, e
                                            )));
                                        }
                                    },
                                    DebugCommand::Step | DebugCommand::ShadowStep => {
                                        match debug_manager.step(&mut core) {
                                            Ok(info) => {
                                                halt_pcs.push((name.clone(), info.pc));
                                                let _ =
                                                    evt_tx.send(DebugEvent::Halted { pc: info.pc });
                                            }
                                            Err(e) => {
                                                let _ = evt_tx.send(DebugEvent::Error(format!(
                                                    "Failed to step {}: {}",
                                                    name, e
                                                )));
                                            }
                                        }
                                    }
                                    DebugCommand::StepOver => {
                                        if let Some(debug_info) = symbol_manager.debug_info() {
                                            match SteppingMode::OverStatement
                                                .step(&mut core, debug_info)
                                            {
                                                Ok((_status, pc)) => {
                                                    halt_pcs.push((name.clone(), pc));
                                                    let _ = evt_tx.send(DebugEvent::Halted { pc });
                                                }
                                                Err(e) => {
                                                    let _ =
                                                        evt_tx.send(DebugEvent::Error(format!(
                                                            "StepOver failed for {}: {:?}",
                                                            name, e
                                                        )));
                                                }
                                            }
                                        }
                                    }
                                    DebugCommand::StepInto => {
                                        if let Some(debug_info) = symbol_manager.debug_info() {
                                            match SteppingMode::IntoStatement
                                                .step(&mut core, debug_info)
                                            {
                                                Ok((_status, pc)) => {
                                                    halt_pcs.push((name.clone(), pc));
                                                    let _ = evt_tx.send(DebugEvent::Halted { pc });
                                                }
                                                Err(e) => {
                                                    let _ =
                                                        evt_tx.send(DebugEvent::Error(format!(
                                                            "StepInto failed for {}: {:?}",
                                                            name, e
                                                        )));
                                                }
                                            }
                                        }
                                    }
                                    DebugCommand::StepOut => {
                                        if let Some(debug_info) = symbol_manager.debug_info() {
                                            match SteppingMode::OutOfStatement
                                                .step(&mut core, debug_info)
                                            {
                                                Ok((_status, pc)) => {
                                                    halt_pcs.push((name.clone(), pc));
                                                    let _ = evt_tx.send(DebugEvent::Halted { pc });
                                                }
                                                Err(e) => {
                                                    let _ =
                                                        evt_tx.send(DebugEvent::Error(format!(
                                                            "StepOut failed for {}: {:?}",
                                                            name, e
                                                        )));
                                                }
                                            }
                                        }
                                    }
                                    DebugCommand::Reset => {
                                        match core.reset_and_halt(Duration::from_millis(100)) {
                                            Ok(_) => {
                                                if let Ok(pc_val) =
                                                    core.read_core_reg(core.program_counter())
                                                {
                                                    halt_pcs.push((name.clone(), pc_val));
                                                    let _ = evt_tx
                                                        .send(DebugEvent::Halted { pc: pc_val });
                                                }
                                            }
                                            Err(e) => {
                                                let _ = evt_tx.send(DebugEvent::Error(format!(
                                                    "Reset failed for {}: {}",
                                                    name, e
                                                )));
                                            }
                                        }
                                    }
                                    _ => {
                                        // For state-less or inspection commands, only run on one target
                                        // (usually the first one in target_names which is active_target)
                                        match &core_cmd {
                                            DebugCommand::ReadMemory(addr, size) => {
                                                let mut data = vec![0u8; *size];
                                                match core.read(*addr, &mut data) {
                                                    Ok(_) => {
                                                        let _ = evt_tx.send(
                                                            DebugEvent::MemoryData(*addr, data),
                                                        );
                                                    }
                                                    Err(e) => {
                                                        let _ = evt_tx
                                                            .send(DebugEvent::Error(e.to_string()));
                                                    }
                                                }
                                            }
                                            DebugCommand::WriteMemory(addr, data) => {
                                                let _ = core.write_8(*addr, data);
                                            }
                                            DebugCommand::ReadRegister(id) => {
                                                if let Ok(val) = core.read_core_reg(*id) {
                                                    let v = match val {
                                                        probe_rs::RegisterValue::U32(v) => v as u64,
                                                        probe_rs::RegisterValue::U64(v) => v,
                                                        probe_rs::RegisterValue::U128(v) => {
                                                            v as u64
                                                        }
                                                    };
                                                    let _ = evt_tx
                                                        .send(DebugEvent::RegisterValue(*id, v));
                                                }
                                            }
                                            DebugCommand::WriteRegister(id, val) => {
                                                let _ = core.write_core_reg(*id, *val);
                                            }
                                            DebugCommand::Disassemble(addr, count) => {
                                                let mut code = vec![0u8; count * 4];
                                                if core.read(*addr, &mut code).is_ok() {
                                                    if let Some(ref a) = arch {
                                                        if let Ok(lines) = disasm_manager
                                                            .disassemble(a, &code, *addr)
                                                        {
                                                            let _ = evt_tx.send(
                                                                DebugEvent::Disassembly(lines),
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                            DebugCommand::SetBreakpoint(addr) => {
                                                let _ = breakpoint_manager
                                                    .set_breakpoint(&mut core, *addr);
                                                let _ = evt_tx.send(DebugEvent::Breakpoints(
                                                    breakpoint_manager.list(),
                                                ));
                                            }
                                            DebugCommand::ClearBreakpoint(addr) => {
                                                let _ = breakpoint_manager
                                                    .clear_breakpoint(&mut core, *addr);
                                                let _ = evt_tx.send(DebugEvent::Breakpoints(
                                                    breakpoint_manager.list(),
                                                ));
                                            }
                                            DebugCommand::ReadPeripheralValues(name) => {
                                                if let Ok(regs) = svd_manager
                                                    .read_peripheral_values(name, &mut core)
                                                {
                                                    let _ =
                                                        evt_tx.send(DebugEvent::Registers(regs));
                                                }
                                            }
                                            DebugCommand::WritePeripheralField {
                                                peripheral,
                                                register,
                                                field,
                                                value,
                                            } => {
                                                let _ = svd_manager.write_peripheral_field(
                                                    &mut core, peripheral, register, field, *value,
                                                );
                                                if let Ok(regs) = svd_manager
                                                    .read_peripheral_values(peripheral, &mut core)
                                                {
                                                    let _ =
                                                        evt_tx.send(DebugEvent::Registers(regs));
                                                }
                                            }
                                            DebugCommand::RttAttach => {
                                                if let Err(e) = rtt_manager.attach(&mut core) {
                                                    let _ = evt_tx.send(DebugEvent::Error(
                                                        format!("RTT attach failed: {}", e),
                                                    ));
                                                } else {
                                                    let _ = evt_tx.send(DebugEvent::RttChannels {
                                                        up_channels: rtt_manager.get_up_channels(),
                                                        down_channels: rtt_manager
                                                            .get_down_channels(),
                                                    });
                                                }
                                            }
                                            DebugCommand::RttWrite { channel, data } => {
                                                let _ = rtt_manager
                                                    .write_channel(&mut core, *channel, data);
                                            }
                                            DebugCommand::GetTasks => {
                                                if let Some(rtos) = &mut rtos_manager {
                                                    if let Ok(tasks) =
                                                        rtos.get_tasks(&mut core, &symbol_manager)
                                                    {
                                                        let _ =
                                                            evt_tx.send(DebugEvent::Tasks(tasks));
                                                    }
                                                }
                                            }
                                            DebugCommand::GetStack => {
                                                if let Ok(frames) = crate::stack::unwind_stack(
                                                    &mut core,
                                                    &symbol_manager,
                                                ) {
                                                    let _ = evt_tx.send(DebugEvent::Stack(frames));
                                                }
                                            }
                                            DebugCommand::WatchVariable(name) => {
                                                if let Some(addr) =
                                                    symbol_manager.lookup_symbol(name)
                                                {
                                                    if let Some(info) = symbol_manager
                                                        .resolve_variable(&mut core, name, addr)
                                                    {
                                                        let _ = evt_tx.send(
                                                            DebugEvent::VariableResolved(info),
                                                        );
                                                    }
                                                }
                                            }
                                            DebugCommand::PollStatus => {
                                                core_status = None;
                                            }
                                            _ => {}
                                        }
                                        // Break after first target for inspection commands
                                        break;
                                    }
                                }
                            }

                            // Perform Parity Check if synced and both halted
                            if let Some((ref m_name, ref s_name)) = shadow_sync {
                                if halt_pcs.len() == 2 {
                                    let mut m_pc = 0;
                                    let mut s_pc = 0;
                                    for (n, pc) in &halt_pcs {
                                        if n == m_name {
                                            m_pc = *pc;
                                        }
                                        if n == s_name {
                                            s_pc = *pc;
                                        }
                                    }

                                    if m_pc != s_pc {
                                        let _ = evt_tx.send(DebugEvent::ParityDiverged {
                                            location: m_pc,
                                            master_val: m_pc,
                                            slave_val: s_pc,
                                            info: "Program Counter Divergence".to_string(),
                                        });
                                    } else {
                                        // 2. Register Check (Exhaustive)
                                        let mut diverged = false;

                                        // Check R0-R12, SP, LR, PC (register indices 0-15)
                                        for reg_idx in 0..16 {
                                            let mut m_val = None;
                                            if let Some(s_m) = sessions.get_mut(m_name) {
                                                if let Ok(mut c_m) = s_m.core(0) {
                                                    if let Ok(v) = c_m.read_core_reg(reg_idx) {
                                                        m_val = Some(match v {
                                                            probe_rs::RegisterValue::U32(v) => {
                                                                v as u64
                                                            }
                                                            probe_rs::RegisterValue::U64(v) => v,
                                                            _ => 0,
                                                        });
                                                    }
                                                }
                                            }

                                            if let Some(mv) = m_val {
                                                if let Some(s_s) = sessions.get_mut(s_name) {
                                                    if let Ok(mut c_s) = s_s.core(0) {
                                                        if let Ok(v) = c_s.read_core_reg(reg_idx) {
                                                            let sv = match v {
                                                                probe_rs::RegisterValue::U32(v) => {
                                                                    v as u64
                                                                }
                                                                probe_rs::RegisterValue::U64(v) => {
                                                                    v
                                                                }
                                                                _ => 0,
                                                            };
                                                            if mv != sv {
                                                                let _ = evt_tx.send(DebugEvent::ParityDiverged {
                                                                    location: m_pc,
                                                                    master_val: mv,
                                                                    slave_val: sv,
                                                                    info: format!("Register R{} Divergence", reg_idx),
                                                                });
                                                                diverged = true;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            if diverged {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            continue;
                        }
                        DebugCommand::LoadSymbols(path) => {
                            if let Err(e) = symbol_manager.load_elf(&path) {
                                let _ = evt_tx.send(DebugEvent::Error(format!(
                                    "Failed to load symbols: {}",
                                    e
                                )));
                            } else {
                                let _ = evt_tx.send(DebugEvent::SymbolsLoaded);
                                rtos_manager =
                                    Some(Box::new(crate::rtos::freertos::FreeRtos::new()));
                            }
                            continue;
                        }
                        DebugCommand::LoadSvd(path) => {
                            if let Err(e) = svd_manager.load_svd(path) {
                                let _ = evt_tx.send(DebugEvent::Error(e.to_string()));
                            } else {
                                let _ = evt_tx.send(DebugEvent::SvdLoaded);
                            }
                            continue;
                        }
                        DebugCommand::GetPeripherals => {
                            let _ = evt_tx
                                .send(DebugEvent::Peripherals(svd_manager.get_peripherals_info()));
                            continue;
                        }
                        DebugCommand::AddPlot { name, var_type } => {
                            if let Some(address) = symbol_manager.lookup_symbol(&name) {
                                plots.push(PlotConfig { name, address, var_type });
                            }
                            continue;
                        }
                        DebugCommand::RemovePlot(name) => {
                            plots.retain(|p| p.name != name);
                            continue;
                        }
                        _ => {}
                    }
                } else {
                    // 3. Polling (Status, RTT, Plots for active_target)
                    if let Some(s) = sessions.get_mut(&active_target) {
                        if let Ok(mut core) = s.core(0) {
                            // Poll Status
                            if let Ok(status) = core.status() {
                                if core_status != Some(status) {
                                    core_status = Some(status);
                                    let _ = evt_tx.send(DebugEvent::Status(status));
                                    if status.is_halted() {
                                        if let Ok(pc) = core.read_core_reg(core.program_counter()) {
                                            let pc_val = match pc {
                                                probe_rs::RegisterValue::U32(v) => v as u64,
                                                probe_rs::RegisterValue::U64(v) => v,
                                                _ => 0,
                                            };
                                            let _ = evt_tx.send(DebugEvent::Halted { pc: pc_val });
                                        }
                                    }
                                }
                            }

                            // Poll RTT
                            if rtt_manager.is_attached() {
                                for ch in rtt_manager.get_up_channels() {
                                    if let Ok(data) = rtt_manager.read_channel(&mut core, ch.number)
                                    {
                                        if !data.is_empty() {
                                            let _ =
                                                evt_tx.send(DebugEvent::RttData(ch.number, data));
                                        }
                                    }
                                }
                            }

                            // Poll Plots
                            if last_plot_poll.elapsed() >= Duration::from_millis(100) {
                                for plot in &plots {
                                    let val = match plot.var_type {
                                        crate::VarType::U32 => {
                                            core.read_word_32(plot.address).ok().map(|v| v as f64)
                                        }
                                        crate::VarType::F32 => core
                                            .read_word_32(plot.address)
                                            .ok()
                                            .map(|v| f32::from_bits(v) as f64),
                                        _ => None,
                                    };
                                    if let Some(v) = val {
                                        let _ = evt_tx.send(DebugEvent::PlotData {
                                            name: plot.name.clone(),
                                            timestamp: session_start.elapsed().as_secs_f64(),
                                            value: v,
                                        });
                                    }
                                }
                                last_plot_poll = Instant::now();
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_millis(10));
            }
        });

        Ok(Self { command_tx: cmd_tx, event_tx: evt_tx, thread_handle: Some(thread_handle) })
    }

    #[cfg(not(feature = "hardware"))]
    pub fn new(_session: Option<crate::probe_rs::Session>) -> Result<Self> {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let (evt_tx, _) = tokio::sync::broadcast::channel(100);

        let thread_handle = thread::spawn(move || loop {
            if let Ok(cmd) = cmd_rx.recv() {
                if matches!(cmd, DebugCommand::Exit) {
                    return;
                }
            }
        });

        Ok(Self { command_tx: cmd_tx, event_tx: evt_tx, thread_handle: Some(thread_handle) })
    }

    pub fn send(&self, cmd: DebugCommand) -> Result<()> {
        self.command_tx.send(cmd).context("Failed to send command")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_session_handle_send_receive() {
        let (handle, cmd_rx, event_tx) = SessionHandle::new_test();

        // Test Command sending
        handle.send(DebugCommand::Halt).unwrap();
        let cmd = cmd_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert!(matches!(cmd, DebugCommand::Halt));

        // Test Event broadcasting
        let mut receiver = handle.subscribe();
        event_tx.send(DebugEvent::Resumed).unwrap();

        let event = receiver.recv().await.unwrap();
        assert!(matches!(event, DebugEvent::Resumed));
    }

    #[test]
    fn test_debug_event_clone() {
        let event = DebugEvent::Halted { pc: 0x1234 };
        let cloned = event.clone();
        if let DebugEvent::Halted { pc } = cloned {
            assert_eq!(pc, 0x1234);
        } else {
            panic!("Clone failed");
        }
    }
}
