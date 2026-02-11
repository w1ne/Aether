//! Session management module.
//!
//! Handles the debug session in a background thread, processing commands
//! and sending events back to the main thread.

use crate::debug::DebugManager;
use crate::VarType;
use anyhow::{Context as _, Result};
use crossbeam_channel::{Receiver, Sender};
use probe_rs::{CoreStatus, Session, MemoryInterface};
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
    Exit,
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
    RegisterValue { address: u16, value: u64 },
    MemoryContent { address: u64, data: Vec<u8> },
    Disassembly(Vec<crate::disasm::InstructionInfo>),
    Breakpoints(Vec<u64>),
    SvdLoaded,
    Peripherals(Vec<crate::svd::PeripheralInfo>),
    Registers(Vec<crate::svd::RegisterInfo>),
    SymbolsLoaded,
    SourceLocation(crate::symbols::SourceInfo),
    BreakpointLocations(Vec<crate::symbols::SourceInfo>),
    RttAttached {
        up_channels: Vec<crate::rtt::RttChannelInfo>,
        down_channels: Vec<crate::rtt::RttChannelInfo>,
    },
    RttData {
        channel: usize,
        data: Vec<u8>,
    },
    PlotData {
        name: String,
        timestamp: f64,
        value: f64,
    },
    Status(CoreStatus),
    Error(String),
}

/// A handle to the debug session running in a background thread.
pub struct SessionHandle {
    command_tx: Sender<DebugCommand>,
    event_tx: tokio::sync::broadcast::Sender<DebugEvent>,
    #[allow(dead_code)] // Kept for future graceful shutdown
    thread_handle: Option<thread::JoinHandle<()>>,
}

    /// Subscribe to debug events
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<DebugEvent> {
        self.event_tx.subscribe()
    }
}

impl SessionHandle {
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
            let mut _last_poll = Instant::now();
            
            let mut plots: Vec<PlotConfig> = Vec::new();
            let mut last_plot_poll = Instant::now();
            let session_start = Instant::now();
            
            let arch = format!("{:?}", session.target().architecture());

            let mut core = match session.core(0) {
                Ok(c) => c,
                Err(e) => {
                    let _ = evt_tx.send(DebugEvent::Error(format!("Failed to attach to core 0: {}", e)));
                    return;
                }
            };

            loop {
                // Process all pending commands
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        DebugCommand::Halt => {
                            match debug_manager.halt(&mut core) {
                                Ok(info) => {
                                    let _ = evt_tx.send(DebugEvent::Halted { pc: info.pc });
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!("Failed to halt: {}", e)));
                                }
                            }
                        }
                        DebugCommand::Resume => {
                            match debug_manager.resume(&mut core) {
                                Ok(_) => {
                                    let _ = evt_tx.send(DebugEvent::Resumed);
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!("Failed to resume: {}", e)));
                                }
                            }
                        }
                        DebugCommand::Step => {
                            match debug_manager.step(&mut core) {
                                Ok(info) => {
                                    let _ = evt_tx.send(DebugEvent::Halted { pc: info.pc });
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!("Failed to step: {}", e)));
                                }
                            }
                        }
                        DebugCommand::StepOver => {
                            if let Some(debug_info) = symbol_manager.debug_info() {
                                 match SteppingMode::OverStatement.step(&mut core, debug_info) {
                                     Ok((status, pc)) => {
                                         let _ = evt_tx.send(DebugEvent::Halted { pc });
                                     }
                                     Err(e) => {
                                         let _ = evt_tx.send(DebugEvent::Error(format!("StepOver failed: {:?}", e)));
                                     }
                                 }
                            } else {
                                 let _ = evt_tx.send(DebugEvent::Error("No symbols loaded for StepOver".to_string()));
                            }
                        }
                        DebugCommand::StepInto => {
                            if let Some(debug_info) = symbol_manager.debug_info() {
                                 match SteppingMode::IntoStatement.step(&mut core, debug_info) {
                                     Ok((status, pc)) => {
                                         let _ = evt_tx.send(DebugEvent::Halted { pc });
                                     }
                                     Err(e) => {
                                         let _ = evt_tx.send(DebugEvent::Error(format!("StepInto failed: {:?}", e)));
                                     }
                                 }
                            } else {
                                 let _ = evt_tx.send(DebugEvent::Error("No symbols loaded for StepInto".to_string()));
                            }
                        }
                        DebugCommand::StepOut => {
                            if let Some(debug_info) = symbol_manager.debug_info() {
                                 match SteppingMode::OutOfStatement.step(&mut core, debug_info) {
                                     Ok((status, pc)) => {
                                         let _ = evt_tx.send(DebugEvent::Halted { pc });
                                     }
                                     Err(e) => {
                                         let _ = evt_tx.send(DebugEvent::Error(format!("StepOut failed: {:?}", e)));
                                     }
                                 }
                            } else {
                                 let _ = evt_tx.send(DebugEvent::Error("No symbols loaded for StepOut".to_string()));
                            }
                        }
                        DebugCommand::ReadRegister(addr) => {
                            match debug_manager.read_core_reg(&mut core, addr) {
                                Ok(val) => {
                                    let _ = evt_tx.send(DebugEvent::RegisterValue { address: addr, value: val });
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!("Failed to read reg {}: {}", addr, e)));
                                }
                            }
                        }
                        DebugCommand::WriteRegister(addr, val) => {
                            if let Err(e) = debug_manager.write_core_reg(&mut core, addr, val) {
                                let _ = evt_tx.send(DebugEvent::Error(format!("Failed to write reg {}: {}", addr, e)));
                            }
                        }
                        DebugCommand::ReadMemory(addr, size) => {
                            match memory_manager.read_block(&mut core, addr, size) {
                                Ok(data) => {
                                    let _ = evt_tx.send(DebugEvent::MemoryContent { address: addr, data });
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!("Failed to read memory @ 0x{:08X}: {}", addr, e)));
                                }
                            }
                        }
                        DebugCommand::WriteMemory(addr, data) => {
                             if let Err(e) = memory_manager.write_block(&mut core, addr, &data) {
                                let _ = evt_tx.send(DebugEvent::Error(format!("Failed to write memory @ 0x{:08X}: {}", addr, e)));
                            }
                        }
                        DebugCommand::Disassemble(addr, size) => {
                            match memory_manager.read_block(&mut core, addr, size) {
                                Ok(code) => {
                                    match disasm_manager.disassemble(&arch, &code, addr) {
                                        Ok(insns) => {
                                            let _ = evt_tx.send(DebugEvent::Disassembly(insns));
                                        }
                                        Err(e) => {
                                            let _ = evt_tx.send(DebugEvent::Error(format!("Disassembly failed: {}", e)));
                                        }
                                    }
                                }
                                Err(e) => {
                                     let _ = evt_tx.send(DebugEvent::Error(format!("Failed to read code for disassembly @ 0x{:08X}: {}", addr, e)));
                                }
                            }
                        }
                        DebugCommand::SetBreakpoint(addr) => {
                            if let Err(e) = breakpoint_manager.set_breakpoint(&mut core, addr) {
                                let _ = evt_tx.send(DebugEvent::Error(format!("Failed to set breakpoint @ 0x{:08X}: {}", addr, e)));
                            } else {
                                let _ = evt_tx.send(DebugEvent::Breakpoints(breakpoint_manager.list()));
                            }
                        }
                        DebugCommand::ClearBreakpoint(addr) => {
                            if let Err(e) = breakpoint_manager.clear_breakpoint(&mut core, addr) {
                                let _ = evt_tx.send(DebugEvent::Error(format!("Failed to clear breakpoint @ 0x{:08X}: {}", addr, e)));
                            } else {
                                let _ = evt_tx.send(DebugEvent::Breakpoints(breakpoint_manager.list()));
                            }
                        }
                        DebugCommand::ListBreakpoints => {
                             let _ = evt_tx.send(DebugEvent::Breakpoints(breakpoint_manager.list()));
                        }
                        DebugCommand::LoadSvd(path) => {
                            match svd_manager.load_svd(path) {
                                Ok(_) => {
                                    let _ = evt_tx.send(DebugEvent::SvdLoaded);
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!("Failed to load SVD: {}", e)));
                                }
                            }
                        }
                        DebugCommand::GetPeripherals => {
                            let info = svd_manager.get_peripherals_info();
                            let _ = evt_tx.send(DebugEvent::Peripherals(info));
                        }
                        DebugCommand::LoadSymbols(path) => {
                            match symbol_manager.load_elf(&path) {
                                Ok(_) => {
                                    let _ = evt_tx.send(DebugEvent::SymbolsLoaded);
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!("Failed to load symbols: {}", e)));
                                }
                            }
                        }
                        DebugCommand::LookupSource(pc) => {
                            if let Some(info) = symbol_manager.lookup(pc) {
                                let _ = evt_tx.send(DebugEvent::SourceLocation(info));
                            }
                        }
                        DebugCommand::ToggleBreakpointAtSource(file, line) => {
                             if let Some(addr) = symbol_manager.get_address(&file, line) {
                                  // Check if breakpoint exists
                                  let exists = breakpoint_manager.list().contains(&addr);
                                  let res = if exists {
                                      breakpoint_manager.clear_breakpoint(&mut core, addr)
                                  } else {
                                      breakpoint_manager.set_breakpoint(&mut core, addr)
                                  };

                                  match res {
                                     Ok(_) => {
                                         let breakpoints = breakpoint_manager.list();
                                         let _ = evt_tx.send(DebugEvent::Breakpoints(breakpoints.clone()));
                                         
                                         // Resolve source locations for all breakpoints
                                         let locations = breakpoints.iter()
                                             .filter_map(|&addr| symbol_manager.lookup(addr))
                                             .collect();
                                         let _ = evt_tx.send(DebugEvent::BreakpointLocations(locations));
                                     }
                                      Err(e) => {
                                          let _ = evt_tx.send(DebugEvent::Error(format!("Failed to toggle breakpoint at {}:{}: {}", file.display(), line, e)));
                                      }
                                 }
                             } else {
                                  let _ = evt_tx.send(DebugEvent::Error(format!("No address mapping found for {}:{}", file.display(), line)));
                             }
                        }
                        DebugCommand::WritePeripheralField { peripheral, register, field, value } => {
                            match svd_manager.write_peripheral_field(&mut core, &peripheral, &register, &field, value) {
                                Ok(_) => {
                                    // Refresh values after write
                                    if let Ok(regs) = svd_manager.read_peripheral_values(&peripheral, &mut core) {
                                        let _ = evt_tx.send(DebugEvent::Registers(regs));
                                    }
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!("Failed to write field {}: {}", field, e)));
                                }
                            }
                        }
                        DebugCommand::RttAttach => {
                            match rtt_manager.attach(&mut core) {
                                Ok(_) => {
                                    let _ = evt_tx.send(DebugEvent::RttAttached {
                                        up_channels: rtt_manager.get_up_channels(),
                                        down_channels: rtt_manager.get_down_channels(),
                                    });
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!("RTT Attach failed: {}", e)));
                                }
                            }
                        }
                        DebugCommand::RttWrite { channel, data } => {
                            if let Err(e) = rtt_manager.write_channel(&mut core, channel, &data) {
                                let _ = evt_tx.send(DebugEvent::Error(format!("RTT Write failed: {}", e)));
                            }
                        }
                        DebugCommand::GetRegisters(name) => {
                            match svd_manager.get_registers_info(&name) {
                                Ok(regs) => {
                                    let _ = evt_tx.send(DebugEvent::Registers(regs));
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!("Failed to get registers: {} - {}", name, e)));
                                }
                            }
                        }
                        DebugCommand::ReadPeripheralValues(name) => {
                            match svd_manager.read_peripheral_values(&name, &mut core) {
                                Ok(regs) => {
                                    let _ = evt_tx.send(DebugEvent::Registers(regs));
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!("Failed to read peripheral {}: {}", name, e)));
                                }
                            }
                        }
                        DebugCommand::PollStatus => {
                             match debug_manager.status(&mut core) {
                                Ok(status) => {
                                    let _ = evt_tx.send(DebugEvent::Status(status));
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(DebugEvent::Error(format!("Failed to get status: {}", e)));
                                }
                            }
                        }
                        DebugCommand::AddPlot { name, var_type } => {
                             if let Some(address) = symbol_manager.lookup_symbol(&name) {
                                  plots.push(PlotConfig { name: name.clone(), address, var_type });
                             } else {
                                  // Try parsing as address
                                  if let Ok(address) = u64::from_str_radix(name.trim_start_matches("0x"), 16) {
                                      plots.push(PlotConfig { name: name.clone(), address, var_type });
                                  } else {
                                      let _ = evt_tx.send(DebugEvent::Error(format!("Symbol not found: {}", name)));
                                  }
                             }
                        }
                        DebugCommand::RemovePlot(name) => {
                             plots.retain(|p| p.name != name);
                        }
                        DebugCommand::Exit => return,
                    }
                }

                // Periodic RTT Polling
                if rtt_manager.is_attached() {
                    let up_channels = rtt_manager.get_up_channels();
                    for chan in up_channels {
                        match rtt_manager.read_channel(&mut core, chan.number) {
                            Ok(data) => {
                                if !data.is_empty() {
                                    let _ = evt_tx.send(DebugEvent::RttData { 
                                        channel: chan.number, 
                                        data 
                                    });
                                }
                            }
                            Err(_) => {
                                // Don't spam errors on every poll
                            }
                        }
                    }
                }

                // Periodic Plot Polling
                if last_plot_poll.elapsed() >= Duration::from_millis(50) {
                     last_plot_poll = Instant::now();
                     let timestamp = session_start.elapsed().as_secs_f64();
                     
                     for plot in &plots {
                          let val_res = match plot.var_type {
                              VarType::U8 => core.read_word_8(plot.address).map(|v| v as f64),
                              VarType::U16 => core.read_word_16(plot.address).map(|v| v as f64),
                              VarType::U32 => core.read_word_32(plot.address).map(|v| v as f64),
                              VarType::U64 => core.read_word_64(plot.address).map(|v| v as f64),
                              VarType::I8 => core.read_word_8(plot.address).map(|v| v as i8 as f64),
                              VarType::I16 => core.read_word_16(plot.address).map(|v| v as i16 as f64),
                              VarType::I32 => core.read_word_32(plot.address).map(|v| v as i32 as f64),
                              VarType::I64 => core.read_word_64(plot.address).map(|v| v as i64 as f64),
                              VarType::F32 => core.read_word_32(plot.address).map(|v| f32::from_bits(v as u32) as f64),
                              VarType::F64 => core.read_word_64(plot.address).map(|v| f64::from_bits(v)),
                          };
                          
                          match val_res {
                              Ok(value) => {
                                   let _ = evt_tx.send(DebugEvent::PlotData { 
                                       name: plot.name.clone(), 
                                       timestamp, 
                                       value 
                                   });
                              }
                              Err(_) => {} // Ignore errors during polling
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
