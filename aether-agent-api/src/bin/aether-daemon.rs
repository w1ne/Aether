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
                        _ => {}
                    }
                }
            }
        });
        
        Arc::new(handle)
    } else {
        // 1. Connect to Probe
        let probe_manager = ProbeManager::new();
        let probes = probe_manager.list_probes()?;

        if probes.is_empty() {
            error!("No debug probes found!");
            return Ok(());
        }

        if args.probe_index >= probes.len() {
            error!("Probe index {} out of range (found {} probes)", args.probe_index, probes.len());
            return Ok(());
        }

        info!("Connecting to probe: {}", probes[args.probe_index].name());
        let probe = probe_manager.open_probe(args.probe_index)?;
        
        info!("Detecting target...");
        let (target, session) = probe_manager.detect_target(probe)?;
        info!("Attached to target: {}", target.name);

        // 2. Create Session Handle
        Arc::new(SessionHandle::new(session)?)
    };

    // 3. Start Server
    info!("Starting gRPC server on {}:{}", args.host, args.port);
    
    // Handle Ctrl+C
    let server_handle = session_handle.clone();
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
