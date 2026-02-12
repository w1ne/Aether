//! Session management module.
//!
//! Handles the debug session in a background thread, processing commands
//! and sending events back to the main thread.

use crate::debug::DebugManager;
use crate::VarType;
use anyhow::{Context as _, Result};
use crossbeam_channel::{Receiver, Sender};
use probe_rs::{CoreStatus, Session, MemoryInterface};
use probe_rs::flashing::{FlashProgress, ProgressEvent};
use probe_rs_debug::SteppingMode;
use std::thread;
use std::time::{Duration, Instant};

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
    AddPlot { name: String, var_type: VarType },
    RemovePlot(String),
    GetTasks,
    GetStack,
    EnableTrace(crate::trace::TraceConfig),
    Exit,
    StartFlashing(std::path::PathBuf),
}

struct PlotConfig {
    name: String,
    address: u64,
    var_type: VarType,
}

#[derive(Debug, Clone)]
pub enum DebugEvent {
    Halted { pc: u64 },
    Resumed,
    RegisterValue(u16, u64),
    MemoryData(u64, Vec<u8>), // Renamed from MemoryContent to match usage
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
    Tasks(Vec<crate::TaskInfo>),
    Stack(Vec<crate::stack::StackFrame>),
    TraceData(Vec<u8>),
    Status(CoreStatus),
    Error(String),
    FlashProgress(f32),
    FlashStatus(String),
    FlashDone,
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
    pub fn new_test() -> (Self, Receiver<DebugCommand>, tokio::sync::broadcast::Sender<DebugEvent>) {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let (evt_tx, _) = tokio::sync::broadcast::channel(100);
        
        (
            Self {
                command_tx: cmd_tx,
                event_tx: evt_tx.clone(),
                thread_handle: None,
            },
            cmd_rx,
            evt_tx
        )
    }

    pub fn new(mut session: Session) -> Result<Self> {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        // create a broadcast channel with capacity 100
        let (evt_tx, _) = tokio::sync::broadcast::channel(100);
        let evt_tx_thread = evt_tx.clone();

        let thread_handle = thread::spawn(move || {
            let evt_tx = evt_tx_thread; // Shadow for inner scope
            let debug_manager = DebugManager::new();
            let memory_manager = crate::MemoryManager::new();
            let disasm_manager = crate::disasm::DisassemblyManager::new();
            let mut breakpoint_manager = crate::debug::BreakpointManager::new();
            let mut svd_manager = crate::svd::SvdManager::new();
            let mut rtt_manager = crate::rtt::RttManager::new();
            let mut symbol_manager = crate::symbols::SymbolManager::new();
            let mut trace_manager = crate::trace::TraceManager::new();
            let mut rtos_manager: Option<Box<dyn crate::rtos::RtosAware>> = None;
            let mut _last_poll = Instant::now();
            let mut core_status = None;
            
            let mut plots: Vec<PlotConfig> = Vec::new();
            let mut last_plot_poll = Instant::now();
            
            let arch = format!("{:?}", session.target().architecture());
            let session_start = Instant::now();

            // Loop for processing commands and events
            loop {
                // 1. Trace Polling (needs &mut Session)
                if let Ok(data) = trace_manager.read_data(&mut session) {
                    if !data.is_empty() {
                         let _ = evt_tx.send(DebugEvent::TraceData(data));
                    }
                }

                // 2. Commands (Session or Core)
                let cmd_opt = cmd_rx.try_recv().ok();

                if let Some(cmd) = cmd_opt {
                    match cmd {
                         DebugCommand::EnableTrace(config) => {
                             if let Err(e) = trace_manager.enable(&mut session, config) {
                                  let _ = evt_tx.send(DebugEvent::Error(format!("Failed to enable trace: {}", e)));
                             }
                             continue;
                         }
                         DebugCommand::Exit => return,
                         DebugCommand::StartFlashing(path) => {
                             let flash_manager = crate::flash::FlashManager::new();
                             let tx_clone = evt_tx.clone();
                             
                             let progress = FlashProgress::new(move |event| {
                                 let update = match event {
                                     ProgressEvent::Started(_) => DebugEvent::FlashStatus("Started".to_string()),
                                     ProgressEvent::Progress { size, .. } => DebugEvent::FlashProgress(size as f32), // Placeholder for proper ratio
                                     ProgressEvent::Finished(_) => DebugEvent::FlashDone,
                                     ProgressEvent::Failed(_) => DebugEvent::Error("Flash failed".to_string()),
                                     _ => return,
                                 };
                                 let _ = tx_clone.send(update);
                             });
                             
                             match flash_manager.flash_elf(&mut session, &path, progress) {
                                 Ok(_) => { let _ = evt_tx.send(DebugEvent::FlashDone); }
                                 Err(e) => { let _ = evt_tx.send(DebugEvent::Error(format!("Flash failed: {}", e))); }
                             }
                             continue;
                         }
                         // Core commands
                         core_cmd => {
                             let mut core = match session.core(0) {
                                 Ok(c) => c,
                                 Err(e) => {
                                     let _ = evt_tx.send(DebugEvent::Error(format!("Failed to attach core: {}", e)));
                                     continue;
                                 }
                             };
                             
                             match core_cmd {
                                 DebugCommand::Halt => {
                                     match debug_manager.halt(&mut core) {
                                         Ok(info) => { let _ = evt_tx.send(DebugEvent::Halted { pc: info.pc }); }
                                         Err(e) => { let _ = evt_tx.send(DebugEvent::Error(format!("Failed to halt: {}", e))); }
                                     }
                                 }
                                 DebugCommand::Resume => {
                                     match debug_manager.resume(&mut core) {
                                         Ok(_) => { let _ = evt_tx.send(DebugEvent::Resumed); }
                                         Err(e) => { let _ = evt_tx.send(DebugEvent::Error(format!("Failed to resume: {}", e))); }
                                     }
                                 }
                                 DebugCommand::Step => {
                                     match debug_manager.step(&mut core) {
                                         Ok(info) => { let _ = evt_tx.send(DebugEvent::Halted { pc: info.pc }); }
                                         Err(e) => { let _ = evt_tx.send(DebugEvent::Error(format!("Failed to step: {}", e))); }
                                     }
                                 }
                                 DebugCommand::StepOver => {
                                     if let Some(debug_info) = symbol_manager.debug_info() {
                                         match SteppingMode::OverStatement.step(&mut core, debug_info) {
                                              Ok((_status, pc)) => { let _ = evt_tx.send(DebugEvent::Halted { pc }); }
                                              Err(e) => { let _ = evt_tx.send(DebugEvent::Error(format!("StepOver failed: {:?}", e))); }
                                         }
                                     } else {
                                          let _ = evt_tx.send(DebugEvent::Error("No symbols".to_string()));
                                     }
                                 }
                                 DebugCommand::StepInto => {
                                     if let Some(debug_info) = symbol_manager.debug_info() {
                                         match SteppingMode::IntoStatement.step(&mut core, debug_info) {
                                              Ok((_status, pc)) => { let _ = evt_tx.send(DebugEvent::Halted { pc }); }
                                              Err(e) => { let _ = evt_tx.send(DebugEvent::Error(format!("StepInto failed: {:?}", e))); }
                                         }
                                     } else {
                                          let _ = evt_tx.send(DebugEvent::Error("No symbols".to_string()));
                                     }
                                 }
                                 DebugCommand::StepOut => {
                                      let _ = debug_manager.step(&mut core); 
                                 }
                                 DebugCommand::ReadMemory(addr, size) => {
                                     let mut data = vec![0u8; size];
                                     match core.read(addr, &mut data) {
                                         Ok(_) => { let _ = evt_tx.send(DebugEvent::MemoryData(addr, data)); }
                                         Err(e) => { let _ = evt_tx.send(DebugEvent::Error(e.to_string())); }
                                     }
                                 }
                                 DebugCommand::WriteMemory(addr, data) => {
                                     match core.write_8(addr, &data) {
                                         Ok(_) => {}
                                         Err(e) => { let _ = evt_tx.send(DebugEvent::Error(e.to_string())); }
                                     }
                                 }
                                 DebugCommand::Disassemble(addr, count) => {
                                     let mut code = vec![0u8; count * 4];
                                     if core.read(addr, &mut code).is_ok() {
                                         match disasm_manager.disassemble(&arch, &code, addr) {
                                             Ok(lines) => { let _ = evt_tx.send(DebugEvent::Disassembly(lines)); }
                                             Err(e) => { let _ = evt_tx.send(DebugEvent::Error(e.to_string())); }
                                         }
                                     }
                                 }
                                 DebugCommand::ReadRegister(id) => {
                                     if let Ok(val) = core.read_core_reg(id as u16) {
                                          let val_u64: u64 = match val {
                                              probe_rs::RegisterValue::U32(v) => v as u64,
                                              probe_rs::RegisterValue::U64(v) => v,
                                              probe_rs::RegisterValue::U128(v) => v as u64,
                                          };
                                          let _ = evt_tx.send(DebugEvent::RegisterValue(id, val_u64));
                                     }
                                 }
                                 DebugCommand::WriteRegister(id, val) => {
                                     let _ = core.write_core_reg(id as u16, val);
                                 }
                                 DebugCommand::SetBreakpoint(addr) => {
                                     if let Err(e) = breakpoint_manager.set_breakpoint(&mut core, addr) {
                                          let _ = evt_tx.send(DebugEvent::Error(e.to_string()));
                                     } else {
                                          let _ = evt_tx.send(DebugEvent::Breakpoints(breakpoint_manager.list()));
                                     }
                                 }
                                 DebugCommand::ClearBreakpoint(addr) => {
                                     if let Err(e) = breakpoint_manager.clear_breakpoint(&mut core, addr) {
                                          let _ = evt_tx.send(DebugEvent::Error(e.to_string()));
                                     } else {
                                          let _ = evt_tx.send(DebugEvent::Breakpoints(breakpoint_manager.list()));
                                     }
                                 }
                                 DebugCommand::ListBreakpoints => {
                                      let _ = evt_tx.send(DebugEvent::Breakpoints(breakpoint_manager.list()));
                                 }
                                 DebugCommand::LoadSymbols(path) => {
                                     if let Err(e) = symbol_manager.load_elf(&path) {
                                         let _ = evt_tx.send(DebugEvent::Error(format!("Failed to load symbols: {}", e)));
                                     } else {
                                         let _ = evt_tx.send(DebugEvent::SymbolsLoaded);
                                          if let Some(_elf_data) = symbol_manager.elf_data() {
                                               let rtos = crate::rtos::freertos::FreeRtos::new();
                                               rtos_manager = Some(Box::new(rtos));
                                               log::info!("RTOS awareness initialized");
                                          }
                                     }
                                 }
                                 DebugCommand::ReadPeripheralValues(name) => {
                                     if let Ok(regs) = svd_manager.read_peripheral_values(&name, &mut core) {
                                         let _ = evt_tx.send(DebugEvent::Registers(regs));
                                     }
                                 }
                                 DebugCommand::WritePeripheralField { peripheral, register, field, value } => {
                                     let _ = svd_manager.write_peripheral_field(&mut core, &peripheral, &register, &field, value);
                                     if let Ok(regs) = svd_manager.read_peripheral_values(&peripheral, &mut core) {
                                         let _ = evt_tx.send(DebugEvent::Registers(regs));
                                     }
                                 }
                                 DebugCommand::RttAttach => {
                                     if let Err(e) = rtt_manager.attach(&mut core) {
                                          let _ = evt_tx.send(DebugEvent::Error(format!("RTT attach failed: {}", e)));
                                     } else {
                                           let up_channels = rtt_manager.get_up_channels();
                                           let down_channels = rtt_manager.get_down_channels();
                                           let _ = evt_tx.send(DebugEvent::RttChannels { up_channels, down_channels });
                                     }
                                 }
                                 DebugCommand::RttWrite { channel, data } => {
                                     let _ = rtt_manager.write_channel(&mut core, channel, &data);
                                 }
                                 DebugCommand::AddPlot { name, var_type } => {
                                     if let Some(address) = symbol_manager.lookup_symbol(&name) {
                                         plots.push(PlotConfig { name, address, var_type });
                                     }
                                 }
                                 DebugCommand::RemovePlot(name) => {
                                     plots.retain(|p| p.name != name);
                                 }
                                 DebugCommand::GetTasks => {
                                     if let Some(rtos) = &mut rtos_manager {
                                           match rtos.get_tasks(&mut core, &symbol_manager) {
                                               Ok(tasks) => { let _ = evt_tx.send(DebugEvent::Tasks(tasks)); }
                                               Err(e) => { let _ = evt_tx.send(DebugEvent::Error(format!("Failed to get tasks: {}", e))); }
                                           }
                                     } else {
                                          let _ = evt_tx.send(DebugEvent::Error("RTOS not initialized".to_string()));
                                     }
                                 }
                                 DebugCommand::GetStack => {
                                    match crate::stack::unwind_stack(&mut core, &symbol_manager) {
                                        Ok(frames) => {
                                            let _ = evt_tx.send(DebugEvent::Stack(frames));
                                        }
                                        Err(e) => {
                                            let _ = evt_tx.send(DebugEvent::Error(format!("Stack unwind failed: {}", e)));
                                        }
                                    }
                                }
                                DebugCommand::ToggleBreakpointAtSource(file, line) => {
                                    if let Some(addr) = symbol_manager.get_address(&std::path::Path::new(&file), line) {
                                        let _ = breakpoint_manager.toggle_breakpoint(&mut core, addr);
                                        let _ = evt_tx.send(DebugEvent::Breakpoints(breakpoint_manager.list()));
                                        
                                        // Send locations
                                        let locations = breakpoint_manager.list().iter()
                                             .filter_map(|&a| symbol_manager.lookup(a))
                                             .collect();
                                        let _ = evt_tx.send(DebugEvent::BreakpointLocations(locations));
                                    }
                                }
                                DebugCommand::LookupSource(addr) => {
                                    if let Some(info) = symbol_manager.lookup(addr) {
                                         let _ = evt_tx.send(DebugEvent::SourceLocation(info));
                                    }
                                }
                                DebugCommand::LoadSvd(path) => {
                                    match svd_manager.load_svd(path) {
                                        Ok(_) => { let _ = evt_tx.send(DebugEvent::SvdLoaded); }
                                        Err(e) => { let _ = evt_tx.send(DebugEvent::Error(e.to_string())); }
                                    }
                                }
                                DebugCommand::GetPeripherals => {
                                    let info = svd_manager.get_peripherals_info();
                                    let _ = evt_tx.send(DebugEvent::Peripherals(info));
                                }
                                DebugCommand::GetRegisters(periph) => {
                                    // Not implemented in svd_manager public API?
                                    // Assuming get request triggers read?
                                    // Or just list registers?
                                    // SvdManager::read_peripheral does both.
                                     if let Ok(regs) = svd_manager.read_peripheral_values(&periph, &mut core) {
                                          let _ = evt_tx.send(DebugEvent::Registers(regs));
                                     }
                                }
                                DebugCommand::PollStatus => {
                                     // Handled in polling loop
                                }
                                _ => {}
                             }
                         }
                    }
                } else {
                    // 3. Polling (Status, RTT, Plots)
                    {
                        let mut core = match session.core(0) {
                             Ok(c) => c,
                             Err(_) => continue,
                        };
                        
                        // Poll Status
                        if let Ok(status) = core.status() {
                             let is_halted = status.is_halted();
                             let was_halted = core_status.as_ref().map(|s: &CoreStatus| s.is_halted()) == Some(true);
                             
                             if is_halted && !was_halted {
                                  // Just halted
                                  if let Ok(pc_val) = core.read_core_reg(core.program_counter()) {
                                      let pc: u64 = match pc_val {
                                          probe_rs::RegisterValue::U32(v) => v as u64,
                                          probe_rs::RegisterValue::U64(v) => v,
                                          probe_rs::RegisterValue::U128(v) => v as u64,
                                      };
                                      let _ = evt_tx.send(DebugEvent::Halted { pc });
                                  }
                             }
                             if core_status != Some(status) {
                                  core_status = Some(status);
                                  let _ = evt_tx.send(DebugEvent::Status(status));
                             }
                        }

                        // Poll RTT
                        if rtt_manager.is_attached() {
                            let up_channels: Vec<usize> = rtt_manager.get_up_channels().iter().map(|c| c.number).collect();
                            for ch_num in up_channels {
                                if let Ok(data) = rtt_manager.read_channel(&mut core, ch_num) {
                                    if !data.is_empty() {
                                        let _ = evt_tx.send(DebugEvent::RttData(ch_num, data));
                                    }
                                }
                            }
                        }
                        
                        // Poll Plots (10Hz)
                        if last_plot_poll.elapsed() >= Duration::from_millis(100) {
                             for plot in &plots {
                                   let val = match plot.var_type {
                                       crate::VarType::U32 => core.read_word_32(plot.address).map(|v| v as f64).ok(),
                                       crate::VarType::F32 => {
                                           core.read_word_32(plot.address).ok().map(|v| f32::from_bits(v) as f64)
                                       }
                                       _ => None
                                   };
                                   
                                   if let Some(v) = val {
                                       let _ = evt_tx.send(DebugEvent::PlotData {
                                           name: plot.name.clone(),
                                           timestamp: session_start.elapsed().as_secs_f64(),
                                           value: v
                                       });
                                   }
                             }
                             last_plot_poll = Instant::now();
                        }
                    }
                }

                thread::sleep(Duration::from_millis(10));
            }
        });

        Ok(Self {
            command_tx: cmd_tx,
            event_tx: evt_tx,
            thread_handle: Some(thread_handle),
        })
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
