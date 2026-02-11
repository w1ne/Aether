//! Memory access module.
//!
//! Handles reading and writing to target memory.

use anyhow::{Context, Result};
use probe_rs::{Core, MemoryInterface};

/// Manager for memory operations.
pub struct MemoryManager;

impl MemoryManager {
    pub fn new() -> Self {
        Self
    }

    /// Read a single 32-bit word from memory.
    pub fn read_32(&self, core: &mut Core, address: u64) -> Result<u32> {
        core.read_word_32(address).context("Failed to read 32-bit word")
    }

    /// Read a single 8-bit byte from memory.
    pub fn read_8(&self, core: &mut Core, address: u64) -> Result<u8> {
        core.read_word_8(address).context("Failed to read 8-bit byte")
    }

    /// Read a block of memory.
    pub fn read_block(&self, core: &mut Core, address: u64, size: usize) -> Result<Vec<u8>> {
        let mut data = vec![0u8; size];
        core.read_8(address, &mut data).context("Failed to read memory block")?;
        Ok(data)
    }

    /// Write a single 32-bit word to memory.
    pub fn write_32(&self, core: &mut Core, address: u64, value: u32) -> Result<()> {
        core.write_word_32(address, value).context("Failed to write 32-bit word")
    }

    /// Write a single 8-bit byte to memory.
    pub fn write_8(&self, core: &mut Core, address: u64, value: u8) -> Result<()> {
        core.write_word_8(address, value).context("Failed to write 8-bit byte")
    }

    /// Write a block of memory.
    pub fn write_block(&self, core: &mut Core, address: u64, data: &[u8]) -> Result<()> {
        core.write_8(address, data).context("Failed to write memory block")
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}
