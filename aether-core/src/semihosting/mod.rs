use anyhow::Result;
use probe_rs::{Core, RegisterValue, MemoryInterface};

pub struct SemihostingManager {}

impl SemihostingManager {
    pub fn new() -> Self {
        Self {}
    }

    /// Check if the core is halted due to a semihosting request and handle it.
    /// Returns Some(String) if output was generated (SYS_WRITE0).
    pub fn check_for_semihosting(&mut self, core: &mut Core) -> Result<Option<String>> {
        // 1. Get PC
        let pc_val = core.read_core_reg(core.program_counter())?;
        let pc: u64 = match pc_val {
             RegisterValue::U32(v) => v as u64,
             RegisterValue::U64(v) => v,
             RegisterValue::U128(v) => v as u64,
        };

        // 2. Read instruction at PC
        // We need to know if it's Thumb or ARM.
        // For Cortex-M it's always Thumb? Not necessarily (M0/M3/M4/M7 yes, but A-series no).
        // `aether` focuses on Cortex-M?
        // Let's assume Thumb for now as most common, or check T bit in EPSR?
        // Actually, probe-rs `core.read_word_16` is safer.
        // Semihosting Opcode:
        // ARM: `SVC 0x123456` (0xEF123456)
        // Thumb: `BKPT 0xAB` (0xBEAB)

        // Let's try to read 16 bits.
        let msg = if let Ok(_inst) = core.read_word_8(pc) { // Read 2 bytes?
             // read_word_8 reads 1 byte.
             // We need 2 bytes.
             let mut buf = [0u8; 2];
             core.read(pc, &mut buf)?;
             let inst16 = u16::from_le_bytes(buf);

             if inst16 == 0xBEAB {
                 // Thumb Semihosting
                 self.handle_semihosting(core, pc, 2)?
             } else {
                 // Check ARM?
                 // Read 4 bytes
                 let mut buf4 = [0u8; 4];
                 if core.read(pc, &mut buf4).is_ok() {
                     let inst32 = u32::from_le_bytes(buf4);
                     if inst32 == 0xEF123456 {
                         // ARM Semihosting
                         self.handle_semihosting(core, pc, 4)?
                     } else {
                         None
                     }
                 } else {
                     None
                 }
             }
        } else {
            None
        };

        Ok(msg)
    }

    fn handle_semihosting(&mut self, core: &mut Core, pc: u64, inst_size: u64) -> Result<Option<String>> {
        // ... (omitted op reading, assume correct from context)
        // Re-implementing logic to be safe or just fixing the write line?
        // Replace_file_content replaces the whole block or chunks.
        // I will replace the function handle_semihosting to be safe.

        let r0 = core.read_core_reg(0)?;
        let r1 = core.read_core_reg(1)?;

        let op = match r0 {
            RegisterValue::U32(v) => v,
            RegisterValue::U64(v) => v as u32,
            _ => 0,
        };

        let param = match r1 {
            RegisterValue::U32(v) => v as u64,
            RegisterValue::U64(v) => v,
            _ => 0,
        };

        let mut result = None;

        match op {
            0x04 => { // SYS_WRITE0 (Write string to console)
                 // R1 points to null-terminated string
                 result = Some(self.read_string(core, param)?);
            }
            0x05 => { // SYS_WRITEC (Write character)
                 // R1 points to character
                 let mut buf = [0u8; 1];
                 core.read(param, &mut buf)?;
                 result = Some(String::from_utf8_lossy(&buf).to_string());
            }
            0x18 => { // SYS_EXIT (AngelSWI_Reason_ReportException)
                // This is used by qemu-semihosting to exit.
                // We might want to signal this?
                // For now, just log?
            }
            _ => {
                // Unknown or unhandled op
            }
        }

        // Resume execution: Advance PC
        let new_pc = pc + inst_size;
        core.write_core_reg(core.program_counter(), new_pc)?;

        // Resume
        core.run()?;

        Ok(result)
    }

    fn read_string(&self, core: &mut Core, addr: u64) -> Result<String> {
        let mut out = String::new();
        let mut curr = addr;
        loop {
            let mut buf = [0u8; 1];
            core.read(curr, &mut buf)?;
            if buf[0] == 0 {
                break;
            }
            out.push(buf[0] as char);
            curr += 1;
            if out.len() > 1024 { // Safety limit
                break;
            }
        }
        Ok(out)
    }
}
