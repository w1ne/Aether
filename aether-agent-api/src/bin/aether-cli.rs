use clap::{Parser, Subcommand};
use aether_agent_api::proto::aether_debug_client::AetherDebugClient;
use aether_agent_api::proto::{Empty, ReadRegisterRequest, ReadMemoryRequest, WriteMemoryRequest, BreakpointRequest};

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
    /// Get status
    Status,
    /// Halt the core
    Halt,
    /// Resume the core
    Resume,
    /// Reset the core
    Reset,
    /// Step one instruction
    Step,
    /// Read a register
    Regs {
        #[arg(short, long)]
        num: Option<u32>,
    },
    /// Read memory
    Read {
        address: String, // Hex string
        length: u32,
    },
    /// Write memory
    Write {
        address: String, // Hex string
        data: String,    // Hex string (e.g. "DEADBEEF")
    },
    /// List breakpoints
    Breakpoints,
    /// Set breakpoint
    Break {
        address: String,
    },
    /// Clear breakpoint
    Clear {
        address: String,
    },
    /// Get Stack Trace
    Stack,
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
        Commands::Halt => {
            client.halt(Empty {}).await?;
            println!("Halted.");
        }
        Commands::Resume => {
            client.resume(Empty {}).await?;
            println!("Resumed.");
        }
        Commands::Reset => {
            client.reset(Empty {}).await?;
            println!("Reset.");
        }
        Commands::Step => {
            client.step(Empty {}).await?;
            println!("Stepped.");
        }
        Commands::Regs { num } => {
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
        Commands::Read { address, length } => {
            let addr = parse_hex(&address)?;
            let data = client.read_memory(ReadMemoryRequest { address: addr, length }).await?.into_inner().data;
            println!("0x{:08X}: {:02X?}", addr, data);
        }
        Commands::Write { address, data } => {
            let addr = parse_hex(&address)?;
            let bytes = parse_hex_bytes(&data)?;
            client.write_memory(WriteMemoryRequest { address: addr, data: bytes }).await?;
            println!("Written.");
        }
        Commands::Breakpoints => {
            let bps = client.list_breakpoints(Empty {}).await?.into_inner().addresses;
            for bp in bps {
                println!("BP: 0x{:08X}", bp);
            }
        }
        Commands::Break { address } => {
            let addr = parse_hex(&address)?;
            client.set_breakpoint(BreakpointRequest { address: addr }).await?;
            println!("Breakpoint set at 0x{:08X}", addr);
        }
        Commands::Clear { address } => {
            let addr = parse_hex(&address)?;
            client.clear_breakpoint(BreakpointRequest { address: addr }).await?;
            println!("Breakpoint cleared at 0x{:08X}", addr);
        }
        Commands::Stack => {
            let frames = client.get_stack(Empty {}).await?.into_inner().frames;
            for (i, frame) in frames.iter().enumerate() {
                println!("#{}: 0x{:08X} in {} ({}:{})", i, frame.pc, frame.function_name, frame.file, frame.line);
            }
        }
    }

    Ok(())
}
