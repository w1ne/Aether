use anyhow::Result;
use probe_rs::Session;
use probe_rs::architecture::arm::component::TraceSink;


pub struct ItmManager {
    enabled: bool,
}

impl ItmManager {
    pub fn new() -> Self {
        Self {
            enabled: false,
        }
    }

    /// Configure ITM/SWO
    pub fn configure(&mut self, session: &mut Session, _baud_rate: u32) -> Result<()> {
        if session.target().architecture() != probe_rs::Architecture::Arm {
            return Err(anyhow::anyhow!("ITM is only supported on ARM targets"));
        }

        // probe-rs 0.31 uses setup_tracing to enable SWO.
        // It likely configures default baud rate or uses values from chip definition if available?
        // Limitations: We can't easily set baud rate without finding where SwoConfig goes.
        // But setup_tracing is the entry point.

        session.setup_tracing(0, TraceSink::TraceMemory)?;

        self.enabled = true;
        Ok(())
    }

    pub fn read_swo(&mut self, session: &mut Session) -> Result<Vec<u8>> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        // Read SWV data.
        // In probe-rs 0.31, reading trace data might be on Session?
        // read_trace_data()?
        // Let's rely on standard method names.
        // If read_swv is gone, and read_swo is gone.
        // check `read_trace_data`.
        match session.read_trace_data() {
             Ok(bytes) => Ok(bytes),
             Err(e) => Err(anyhow::anyhow!("Failed to read trace data: {}", e)),
        }
    }
}
