//! Session management module.
//!
//! Handles the debug session in a background thread, processing commands
//! and sending events back to the main thread.

use crate::debug::DebugManager;
use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};
use probe_rs::{CoreStatus, Session};
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum DebugCommand {
    Halt,
    Resume,
    Step,
    ReadRegister(u16),
    WriteRegister(u16, u64),
    ReadMemory(u64, usize),
    WriteMemory(u64, Vec<u8>),
    Disassemble(u64, usize),
    SetBreakpoint(u64),
    ClearBreakpoint(u64),
    ListBreakpoints,
    LoadSvd(std::path::PathBuf),
    GetPeripherals,
    GetRegisters(String),
    ReadPeripheralValues(String),
    PollStatus,
    Exit,
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
    Status(CoreStatus),
    Error(String),
}

/// A handle to the debug session running in a background thread.
pub struct SessionHandle {
    command_tx: Sender<DebugCommand>,
    event_rx: Receiver<DebugEvent>,
    #[allow(dead_code)] // Kept for future graceful shutdown
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl SessionHandle {
    pub fn new(mut session: Session) -> Result<Self> {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let (evt_tx, evt_rx) = crossbeam_channel::unbounded();

        let thread_handle = thread::spawn(move || {
            let debug_manager = DebugManager::new();
            let memory_manager = crate::MemoryManager::new();
            let disasm_manager = crate::disasm::DisassemblyManager::new();
            let mut breakpoint_manager = crate::debug::BreakpointManager::new();
            let mut svd_manager = crate::svd::SvdManager::new();
            
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
                        DebugCommand::Exit => return,
                    }
                }

                // Periodic status polling could go here, but let's rely on explicit PollStatus for now
                // to avoid spamming the channel.
                thread::sleep(Duration::from_millis(10));
            }
        });

        Ok(Self {
            command_tx: cmd_tx,
            event_rx: evt_rx,
            thread_handle: Some(thread_handle),
        })
    }

    pub fn send(&self, cmd: DebugCommand) -> Result<()> {
        self.command_tx.send(cmd).context("Failed to send command")
    }

    pub fn try_recv(&self) -> Option<DebugEvent> {
        self.event_rx.try_recv().ok()
    }
}
