//! Disassembly module.
//!
//! Handles instruction disassembly using Capstone.

use anyhow::{anyhow, Result};
use capstone::prelude::*;

/// Manager for disassembly operations.
pub struct DisassemblyManager;

#[derive(Debug, Clone)]
pub struct InstructionInfo {
    pub address: u64,
    pub mnemonic: String,
    pub op_str: String,
    pub bytes: Vec<u8>,
}

impl DisassemblyManager {
    pub fn new() -> Self {
        Self
    }

    /// Disassemble a block of code.
    pub fn disassemble(
        &self,
        arch: &str,
        code: &[u8],
        address: u64,
    ) -> Result<Vec<InstructionInfo>> {
        let cs = match arch {
            "Armv7em" | "Armv7m" | "Armv6m" => {
                Capstone::new()
                    .arm()
                    .mode(arch::arm::ArchMode::Thumb) // Most Microcontrollers are Thumb
                    .build()
                    .map_err(|e| anyhow!("Failed to create Capstone: {}", e))?
            }
            "Riscv32" => {
                Capstone::new()
                    .riscv()
                    .mode(arch::riscv::ArchMode::RiscV32)
                    .build()
                    .map_err(|e| anyhow!("Failed to create Capstone: {}", e))?
            }
             _ => {
                // Default to ARM Thumb for now if unknown or fallback
                Capstone::new()
                    .arm()
                    .mode(arch::arm::ArchMode::Thumb)
                    .build()
                    .map_err(|e| anyhow!("Failed to create Capstone for {}: {}", arch, e))?
            }
        };

        let instructions = cs
            .disasm_all(code, address)
            .map_err(|e| anyhow!("Failed to disassemble: {}", e))?;

        Ok(instructions
            .iter()
            .map(|insn| InstructionInfo {
                address: insn.address(),
                mnemonic: insn.mnemonic().unwrap_or("???").to_string(),
                op_str: insn.op_str().unwrap_or("").to_string(),
                bytes: insn.bytes().to_vec(),
            })
            .collect())
    }
}

impl Default for DisassemblyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disassemble_arm_thumb() {
        let manager = DisassemblyManager::new();
        // nop in thumb is 0x46C0 (represented as [0xC0, 0x46] in little-endian byte stream for capstone usually? 
        // actually thumb is often handled as 16-bit. Let's try known bytes.)
        // 0x00 0xbf is NOP in Thumb.
        let code = vec![0x00, 0xbf, 0x00, 0xbf]; 
        let insns = manager.disassemble("Armv7m", &code, 0x1000).unwrap();
        
        assert_eq!(insns.len(), 2);
        assert_eq!(insns[0].address, 0x1000);
        assert_eq!(insns[0].mnemonic, "nop");
    }

    #[test]
    fn test_disassemble_riscv() {
        let manager = DisassemblyManager::new();
        // 0x00000013 is nop in RISC-V (i-type, addi x0, x0, 0)
        let code = vec![0x13, 0x00, 0x00, 0x00]; 
        let insns = manager.disassemble("Riscv32", &code, 0x2000).unwrap();
        
        assert_eq!(insns.len(), 1);
        assert_eq!(insns[0].address, 0x2000);
        assert_eq!(insns[0].mnemonic, "nop");
    }
}
