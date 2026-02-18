use clap::{Parser, Subcommand};
use aether_agent_api::proto::aether_debug_client::AetherDebugClient;
use aether_agent_api::proto::{
    Empty, ReadRegisterRequest, ReadMemoryRequest, WriteMemoryRequest, BreakpointRequest,
    WriteRegisterRequest, WatchVariableRequest, PeripheralRequest, PeripheralWriteRequest,
    RttWriteRequest, FileRequest, DisasmRequest, ItmConfig,
    AttachRequest, ProbeList, ProbeInfo
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Server URL
    #[arg(short, long, default_value = "http://[::1]:50051")]
    url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Core control commands (halt, resume, regs, etc.)
    Core {
        #[command(subcommand)]
        cmd: CoreCommands,
    },
    /// Memory operations (read, write)
    Memory {
        #[command(subcommand)]
        cmd: MemoryCommands,
    },
    /// Target operations (flash, load-svd, disasm)
    Target {
        #[command(subcommand)]
        cmd: TargetCommands,
    },
    /// RTOS & Debug high-level info (tasks, stack, watch)
    Rtos {
        #[command(subcommand)]
        cmd: RtosCommands,
    },
    /// Trace & Logging (rtt, semihosting, itm)
    Trace {
        #[command(subcommand)]
        cmd: TraceCommands,
    },
    /// Probe discovery and attachment
    Probe {
        #[command(subcommand)]
        cmd: ProbeCommands,
    },

    // Legacy support for common top-level commands (optional, but keep it clean)
    /// Quick Status
    Status,
}

#[derive(Subcommand)]
enum CoreCommands {
    /// Halt the core
    Halt,
    /// Resume execution
    Resume,
    /// Reset the target
    Reset,
    /// Step one instruction
    Step,
    /// Step Over
    StepOver,
    /// Step Into
    StepInto,
    /// Step Out
    StepOut,
    /// List or read registers
    Regs {
        #[arg(short, long)]
        num: Option<u32>,
    },
    /// Write to a register
    WriteReg {
        num: u32,
        value: String, // Hex
    },
}

#[derive(Subcommand)]
enum MemoryCommands {
    /// Read memory range
    Read {
        address: String,
        length: u32,
    },
    /// Write bytes to memory
    Write {
        address: String,
        data: String, // Hex
    },
}

#[derive(Subcommand)]
enum TargetCommands {
    /// Flash a binary/ELF to the target
    Flash { path: String },
    /// Load SVD file for peripheral decoding
    LoadSvd { path: String },
    /// Load ELF symbols for debugging
    LoadSymbols { path: String },
    /// Disassemble instructions at an address
    Disasm {
        address: String,
        #[arg(default_value_t = 10)]
        count: u32
    },
    /// List active breakpoints
    Breakpoints,
    /// Set a hardware breakpoint
    Break { address: String },
    /// Clear a breakpoint
    Clear { address: String },
    /// Read peripheral register
    ReadPeri {
        peripheral: String,
        register: String,
    },
    /// Write peripheral register field
    WritePeri {
        peripheral: String,
        register: String,
        field: String,
        value: String, // Hex
    },
}

#[derive(Subcommand)]
enum RtosCommands {
    /// List active RTOS tasks
    Tasks,
    /// Get current stack trace
    Stack,
    /// Watch a variable by name
    Watch { name: String },
}

#[derive(Subcommand)]
enum TraceCommands {
    /// Write to RTT channel
    RttWrite { channel: u32, data: String },
    /// Enable Semihosting
    EnableSemihosting,
    /// Enable ITM
    EnableItm {
        #[arg(default_value_t = 115200)]
        baud: u32
    },
}

#[derive(Subcommand)]
pub enum ProbeCommands {
    /// List available debug probes
    List,
    /// Attach to a target
    Attach {
        /// Probe index
        #[arg(default_value_t = 0)]
        index: usize,
        /// Chip name (e.g. STM32L476RGTx or 'auto')
        #[arg(short, long, default_value = "auto")]
        chip: String,
        /// Protocol (swd or jtag)
        #[arg(long)]
        protocol: Option<String>,
        /// Connect under reset
        #[arg(long)]
        under_reset: bool,
    },
}

fn parse_hex(s: &str) -> Result<u64, std::num::ParseIntError> {
    let s = s.trim_start_matches("0x");
    u64::from_str_radix(s, 16)
}

fn parse_hex_bytes(s: &str) -> Result<Vec<u8>, hex::FromHexError> {
    let s = s.trim_start_matches("0x");
    hex::decode(s)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut client = AetherDebugClient::connect(cli.url).await?;

    match cli.command {
        Commands::Status => {
            let status = client.get_status(Empty {}).await?.into_inner();
            println!("Status: {:?}", status);
        }
        Commands::Core { cmd } => match cmd {
            CoreCommands::Halt => { client.halt(Empty {}).await?; println!("Halted."); }
            CoreCommands::Resume => { client.resume(Empty {}).await?; println!("Resumed."); }
            CoreCommands::Reset => { client.reset(Empty {}).await?; println!("Reset."); }
            CoreCommands::Step => { client.step(Empty {}).await?; println!("Stepped."); }
            CoreCommands::StepOver => { client.step_over(Empty {}).await?; println!("Stepped Over."); }
            CoreCommands::StepInto => { client.step_into(Empty {}).await?; println!("Stepped Into."); }
            CoreCommands::StepOut => { client.step_out(Empty {}).await?; println!("Stepped Out."); }
            CoreCommands::Regs { num } => {
                if let Some(n) = num {
                    let val = client.read_register(ReadRegisterRequest { register_number: n }).await?.into_inner().value;
                    println!("R{}: 0x{:08X}", n, val);
                } else {
                    for i in 0..16 {
                        let val = client.read_register(ReadRegisterRequest { register_number: i }).await?.into_inner().value;
                        println!("R{}: 0x{:08X}", i, val);
                    }
                }
            }
            CoreCommands::WriteReg { num, value } => {
                let val = parse_hex(&value)?;
                client.write_register(WriteRegisterRequest { register_number: num, value: val }).await?;
                println!("Written R{}: 0x{:08X}", num, val);
            }
        },
        Commands::Memory { cmd } => match cmd {
            MemoryCommands::Read { address, length } => {
                let addr = parse_hex(&address)?;
                let data = client.read_memory(ReadMemoryRequest { address: addr, length }).await?.into_inner().data;
                println!("0x{:08X}: {:02X?}", addr, data);
            }
            MemoryCommands::Write { address, data } => {
                let addr = parse_hex(&address)?;
                let bytes = parse_hex_bytes(&data)?;
                client.write_memory(WriteMemoryRequest { address: addr, data: bytes }).await?;
                println!("Written.");
            }
        },
        Commands::Target { cmd } => match cmd {
            TargetCommands::Flash { path } => {
                let mut stream = client.flash(FileRequest { path }).await?.into_inner();
                while let Some(p) = stream.message().await? {
                    if !p.error.is_empty() {
                        eprintln!("Error: {}", p.error);
                        std::process::exit(1);
                    } else if p.done {
                        println!("Flash Complete!"); break;
                    } else {
                        println!("[{}] {:.1}%", p.status, p.progress * 100.0);
                    }
                }
            }
            TargetCommands::LoadSvd { path } => { client.load_svd(FileRequest { path }).await?; println!("SVD Loaded."); }
            TargetCommands::LoadSymbols { path } => { client.load_symbols(FileRequest { path }).await?; println!("Symbols Loaded."); }
            TargetCommands::Disasm { address, count } => {
                let addr = parse_hex(&address)?;
                let resp = client.disassemble(DisasmRequest { address: addr, count }).await?.into_inner();
                for line in resp.instructions { println!("{}", line); }
            }
            TargetCommands::Breakpoints => {
                let bps = client.list_breakpoints(Empty {}).await?.into_inner().addresses;
                for bp in bps { println!("BP: 0x{:08X}", bp); }
            }
            TargetCommands::Break { address } => {
                let addr = parse_hex(&address)?;
                client.set_breakpoint(BreakpointRequest { address: addr }).await?;
                println!("Breakpoint set at 0x{:08X}", addr);
            }
            TargetCommands::Clear { address } => {
                let addr = parse_hex(&address)?;
                client.clear_breakpoint(BreakpointRequest { address: addr }).await?;
                println!("Breakpoint cleared at 0x{:08X}", addr);
            }
            TargetCommands::ReadPeri { peripheral, register } => {
                let val = client.read_peripheral(PeripheralRequest { peripheral, register }).await?.into_inner().value;
                println!("Value: 0x{:08X}", val);
            }
            TargetCommands::WritePeri { peripheral, register, field, value } => {
                let val = parse_hex(&value)?;
                client.write_peripheral(PeripheralWriteRequest { peripheral, register, field, value: val }).await?;
                println!("Written.");
            }
        },
        Commands::Rtos { cmd } => match cmd {
            RtosCommands::Tasks => {
                let tasks = client.get_tasks(Empty {}).await?.into_inner().tasks;
                println!("{:<20} {:<10} {:<10} {:<10}", "Name", "State", "Stack", "Type");
                for t in tasks {
                    println!("{:<20} {:<10} {}/{}  {}", t.name, t.state, t.stack_usage, t.stack_size, t.task_type);
                }
            }
            RtosCommands::Stack => {
                let frames = client.get_stack(Empty {}).await?.into_inner().frames;
                for (i, f) in frames.iter().enumerate() {
                    let func = f.function_name.as_deref().unwrap_or("??");
                    let file = f.file.as_deref().unwrap_or("??");
                    let line = f.line.map(|l| l.to_string()).unwrap_or_else(|| "??".to_string());
                    println!("#{}: 0x{:08X} in {} ({}:{})", i, f.pc, func, file, line);
                }
            }
            RtosCommands::Watch { name } => {
                client.watch_variable(WatchVariableRequest { name: name.clone() }).await?;
                println!("Watching variable: {}", name);
            }
        },
        Commands::Trace { cmd } => match cmd {
            TraceCommands::RttWrite { channel, data } => {
                client.rtt_write(RttWriteRequest { channel, data: data.into_bytes() }).await?;
                println!("Sent to RTT ch{}", channel);
            }
            TraceCommands::EnableSemihosting => {
                client.enable_semihosting(Empty {}).await?;
                println!("Semihosting enabled.");
            }
            TraceCommands::EnableItm { baud } => {
                client.enable_itm(ItmConfig { baud_rate: baud }).await?;
                println!("ITM enabled at {} baud.", baud);
            }
        }
        Commands::Probe { cmd } => match cmd {
            ProbeCommands::List => {
                let resp = client.list_probes(Empty {}).await?.into_inner();
                println!("{:<5} {:<20} {:<20}", "Index", "Model", "Serial");
                for p in resp.probes {
                    println!("{:<5} {:<20} {:<20}", p.index, p.name, p.serial);
                }
            }
            ProbeCommands::Attach { index, chip, protocol, under_reset } => {
                println!("Attaching to {} via probe {}...", chip, index);
                client.attach(AttachRequest {
                    probe_index: index as u32,
                    chip,
                    protocol,
                    under_reset,
                }).await?;
                println!("Successfully attached.");
            }
        }
    }

    Ok(())
}
