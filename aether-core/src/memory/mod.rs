//! Memory access module.
//!
//! Handles reading and writing to target memory.

use anyhow::{Context, Result};
use probe_rs::MemoryInterface;

/// Manager for memory operations.
pub struct MemoryManager;

impl MemoryManager {
    pub fn new() -> Self {
        Self
    }

    /// Read a single 32-bit word from memory.
    pub fn read_32(&self, core: &mut dyn MemoryInterface, address: u64) -> Result<u32> {
        core.read_word_32(address).context("Failed to read 32-bit word")
    }

    /// Read a single 8-bit byte from memory.
    pub fn read_8(&self, core: &mut dyn MemoryInterface, address: u64) -> Result<u8> {
        core.read_word_8(address).context("Failed to read 8-bit byte")
    }

    /// Read a block of memory.
    pub fn read_block(&self, core: &mut dyn MemoryInterface, address: u64, size: usize) -> Result<Vec<u8>> {
        let mut data = vec![0u8; size];
        core.read_8(address, &mut data).context("Failed to read memory block")?;
        Ok(data)
    }

    /// Write a single 32-bit word to memory.
    pub fn write_32(&self, core: &mut dyn MemoryInterface, address: u64, value: u32) -> Result<()> {
        core.write_word_32(address, value).context("Failed to write 32-bit word")
    }

    /// Write a single 8-bit byte to memory.
    pub fn write_8(&self, core: &mut dyn MemoryInterface, address: u64, value: u8) -> Result<()> {
        core.write_word_8(address, value).context("Failed to write 8-bit byte")
    }

    /// Write a block of memory.
    pub fn write_block(&self, core: &mut dyn MemoryInterface, address: u64, data: &[u8]) -> Result<()> {
        core.write_8(address, data).context("Failed to write memory block")
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use super::*;

    struct MockMemory {
        data: std::collections::HashMap<u64, u8>,
    }

    impl MockMemory {
        fn new() -> Self {
            Self { data: std::collections::HashMap::new() }
        }
    }

    impl MemoryInterface for MockMemory {
        fn read_word_8(&mut self, address: u64) -> Result<u8, probe_rs::Error> {
            let mut b = [0u8; 1];
            self.read_8(address, &mut b)?;
            Ok(b[0])
        }
        fn read_word_16(&mut self, address: u64) -> Result<u16, probe_rs::Error> {
            let mut b = [0u8; 2];
            self.read_8(address, &mut b)?;
            Ok(u16::from_le_bytes(b))
        }
        fn read_word_32(&mut self, address: u64) -> Result<u32, probe_rs::Error> {
            let mut b = [0u8; 4];
            self.read_8(address, &mut b)?;
            Ok(u32::from_le_bytes(b))
        }
        fn read_word_64(&mut self, address: u64) -> Result<u64, probe_rs::Error> {
            let mut b = [0u8; 8];
            self.read_8(address, &mut b)?;
            Ok(u64::from_le_bytes(b))
        }
        fn write_word_8(&mut self, address: u64, data: u8) -> Result<(), probe_rs::Error> {
            self.write_8(address, &[data])
        }
        fn write_word_16(&mut self, address: u64, data: u16) -> Result<(), probe_rs::Error> {
            self.write_8(address, &data.to_le_bytes())
        }
        fn write_word_32(&mut self, address: u64, data: u32) -> Result<(), probe_rs::Error> {
            self.write_8(address, &data.to_le_bytes())
        }
        fn write_word_64(&mut self, address: u64, data: u64) -> Result<(), probe_rs::Error> {
            self.write_8(address, &data.to_le_bytes())
        }
        fn read_8(&mut self, address: u64, data: &mut [u8]) -> Result<(), probe_rs::Error> {
            for (i, byte) in data.iter_mut().enumerate() {
                *byte = *self.data.get(&(address + i as u64)).unwrap_or(&0);
            }
            Ok(())
        }
        fn write_8(&mut self, address: u64, data: &[u8]) -> Result<(), probe_rs::Error> {
            for (i, &byte) in data.iter().enumerate() {
                self.data.insert(address + i as u64, byte);
            }
            Ok(())
        }
        fn read_16(&mut self, address: u64, data: &mut [u16]) -> Result<(), probe_rs::Error> {
            for (i, word) in data.iter_mut().enumerate() {
                *word = self.read_word_16(address + (i * 2) as u64)?;
            }
            Ok(())
        }
        fn write_16(&mut self, address: u64, data: &[u16]) -> Result<(), probe_rs::Error> {
            for (i, &word) in data.iter().enumerate() {
                self.write_word_16(address + (i * 2) as u64, word)?;
            }
            Ok(())
        }
        fn read_32(&mut self, address: u64, data: &mut [u32]) -> Result<(), probe_rs::Error> {
            for (i, word) in data.iter_mut().enumerate() {
                *word = self.read_word_32(address + (i * 4) as u64)?;
            }
            Ok(())
        }
        fn write_32(&mut self, address: u64, data: &[u32]) -> Result<(), probe_rs::Error> {
            for (i, &word) in data.iter().enumerate() {
                self.write_word_32(address + (i * 4) as u64, word)?;
            }
            Ok(())
        }
        fn read_64(&mut self, address: u64, data: &mut [u64]) -> Result<(), probe_rs::Error> {
            for (i, word) in data.iter_mut().enumerate() {
                *word = self.read_word_64(address + (i * 8) as u64)?;
            }
            Ok(())
        }
        fn write_64(&mut self, address: u64, data: &[u64]) -> Result<(), probe_rs::Error> {
            for (i, &word) in data.iter().enumerate() {
                self.write_word_64(address + (i * 8) as u64, word)?;
            }
            Ok(())
        }
        fn flush(&mut self) -> Result<(), probe_rs::Error> { Ok(()) }
        fn supports_native_64bit_access(&mut self) -> bool { false }
        fn supports_8bit_transfers(&self) -> Result<bool, probe_rs::Error> { Ok(true) }
    }

    #[test]
    fn test_memory_manager_read_write_32() {
        let mut mock = MockMemory::new();
        let mgr = MemoryManager::new();
        
        mgr.write_32(&mut mock, 0x1000, 0xDEADBEEF).unwrap();
        assert_eq!(mgr.read_32(&mut mock, 0x1000).unwrap(), 0xDEADBEEF);
    }

    #[test]
    fn test_memory_manager_read_write_block() {
        let mut mock = MockMemory::new();
        let mgr = MemoryManager::new();
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        
        mgr.write_block(&mut mock, 0x2000, &data).unwrap();
        assert_eq!(mgr.read_block(&mut mock, 0x2000, 8).unwrap(), data);
    }
}
