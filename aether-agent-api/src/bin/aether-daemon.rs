use clap::Parser;
use aether_core::{ProbeManager, SessionHandle};
use std::sync::Arc;
use log::{info, error};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 50051)]
    port: u16,

    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Index of probe to use (default: 0)
    #[arg(long, default_value_t = 0)]
    probe_index: usize,

    /// Run in mock mode (no hardware required)
    #[arg(long)]
    mock: bool,

    /// Chip name (e.g. STM32L476RGTx). Use 'auto' for auto-detection.
    #[arg(short, long, default_value = "auto")]
    chip: String,

    /// Debug protocol (swd, jtag)
    #[arg(long)]
    protocol: Option<String>,

    /// Connect under reset
    #[arg(long)]
    under_reset: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    info!("Starting Aether Daemon...");

    let session_handle = if args.mock {
        info!("Starting in MOCK mode. No hardware will be accessed.");
        let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
        
        // Spawn mock simulation loop
        tokio::task::spawn_blocking(move || {
            use aether_core::DebugCommand;
            use aether_core::DebugEvent;
            
            loop {
                if let Ok(cmd) = cmd_rx.recv() {
                    match cmd {
                        DebugCommand::Halt => {
                            let _ = event_tx.send(DebugEvent::Halted { pc: 0x08000123 });
                        }
                        DebugCommand::Resume => {
                            let _ = event_tx.send(DebugEvent::Resumed);
                        }
                        DebugCommand::PollStatus => {
                            // Automatically sent by some commands, ignore/respond in mock
                        }
                        DebugCommand::ReadRegister(n) => {
                             let _ = event_tx.send(DebugEvent::RegisterValue(n, 0xFACEFEED));
                        }
                        DebugCommand::ReadMemory(addr, len) => {
                             let data = vec![0xAA; len];
                             let _ = event_tx.send(DebugEvent::MemoryData(addr, data));
                        }
                        DebugCommand::Step | DebugCommand::StepOver | DebugCommand::StepInto | DebugCommand::StepOut => {
                            let _ = event_tx.send(DebugEvent::Halted { pc: 0x08000124 });
                        }
                        DebugCommand::WriteRegister(_, _) => {
                             // Mock write success (no event needed usually, or maybe RegisterValue?)
                        }
                        DebugCommand::GetTasks => {
                            let tasks = vec![
                                aether_core::TaskInfo {
                                    name: "Idle".to_string(),
                                    priority: 0,
                                    state: aether_core::TaskState::Running,
                                    stack_usage: 100,
                                    stack_size: 200,
                                    handle: 0,
                                    task_type: aether_core::TaskType::Thread,
                                },
                            ];
                            let _ = event_tx.send(DebugEvent::Tasks(tasks));
                        }
                        DebugCommand::RttWrite { channel, data } => {
                             // Echo back on same channel?
                             let _ = event_tx.send(DebugEvent::RttData(channel, data));
                        }
                        DebugCommand::StartFlashing(_) => {
                            // Simulate flashing sequence
                            let _ = event_tx.send(DebugEvent::FlashStatus("Erasing...".to_string()));
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            let _ = event_tx.send(DebugEvent::FlashProgress(0.5));
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            let _ = event_tx.send(DebugEvent::FlashDone);
                        }
                        DebugCommand::Disassemble(addr, count) => {
                             let mut lines = Vec::new();
                             for i in 0..count {
                                 lines.push(aether_core::disasm::InstructionInfo {
                                     address: addr + (i as u64 * 4),
                                     mnemonic: "mov".to_string(),
                                     op_str: format!("r{}, r{}", i, i+1),
                                     bytes: vec![0x00, 0xbf],
                                 });
                             }
                             let _ = event_tx.send(DebugEvent::Disassembly(lines));
                        }
                        DebugCommand::LoadSvd(_) => {
                            let _ = event_tx.send(DebugEvent::SvdLoaded);
                        }
                        DebugCommand::LoadSymbols(_) => {
                             let _ = event_tx.send(DebugEvent::SymbolsLoaded);
                        }
                        DebugCommand::ReadPeripheralValues(_name) => {
                             let regs = vec![
                                 aether_core::svd::RegisterInfo {
                                     name: "CR".to_string(),
                                     address_offset: 0x0,
                                     size: 32,
                                     value: Some(0x1),
                                     fields: vec![],
                                     description: Some("Control Register".to_string()),
                                 },
                             ];
                             let _ = event_tx.send(DebugEvent::Registers(regs));
                        }
                        DebugCommand::WatchVariable(name) => {
                             let _ = event_tx.send(DebugEvent::VariableResolved(aether_core::symbols::TypeInfo {
                                 name: name.clone(),
                                 value_formatted_string: "42".to_string(),
                                 kind: "Primitive".to_string(),
                                 members: None,
                                 address: Some(0x20000000),
                             }));
                        }
                        DebugCommand::Reset => {
                             let _ = event_tx.send(DebugEvent::Halted { pc: 0x08000000 });
                        }
                        DebugCommand::SetBreakpoint(addr) => {
                             let _ = event_tx.send(DebugEvent::Breakpoints(vec![addr]));
                        }
                        DebugCommand::ClearBreakpoint(_) => {
                             let _ = event_tx.send(DebugEvent::Breakpoints(vec![]));
                        }
                        _ => {}
                    }
                }
            }
        });
        
        Arc::new(handle)
    } else {
        // 1. Initial Connection (Optional)
        let mut session = None;
        
        // Only try to connect if the user provided something beyond the defaults
        // OR if they want us to try auto-discovery immediately.
        // For "zero-config", we start disconnected and let the user attach later.
        if args.chip != "auto" {
            let probe_manager = ProbeManager::new();
            let probes = probe_manager.list_probes()?;

            if !probes.is_empty() && args.probe_index < probes.len() {
                let protocol = match args.protocol.as_deref() {
                    Some("swd") => Some(aether_core::WireProtocol::Swd),
                    Some("jtag") => Some(aether_core::WireProtocol::Jtag),
                    _ => None,
                };

                info!("Attempting initial connection to target: {}...", args.chip);
                    
                match probe_manager.connect(
                    args.probe_index, 
                    &args.chip, 
                    protocol, 
                    args.under_reset,
                ) {
                    Ok((target, s)) => {
                        info!("Attached to target: {}", target.name);
                        session = Some(s);
                    }
                    Err(e) => {
                        error!("Initial attachment failed: {}. Starting in disconnected mode.", e);
                    }
                }
            }
        } else {
            info!("Starting in zero-config mode. Use 'attach' command to connect to a target.");
        }

        // 2. Create Session Handle
        Arc::new(SessionHandle::new(session)?)
    };

    // 3. Start Server
    info!("Starting gRPC server on {}:{}", args.host, args.port);
    
    // Handle Ctrl+C
    let _server_handle = session_handle.clone();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Shutting down...");
                std::process::exit(0);
            },
            Err(err) => {
                error!("Unable to listen for shutdown signal: {}", err);
            },
        }
    });

    aether_agent_api::run_server(session_handle, &args.host, args.port).await?;

    Ok(())
}
