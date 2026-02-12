use anyhow::{Context as _, Result};
use probe_rs::Core;
use probe_rs::rtt::Rtt;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RttChannelInfo {
    pub number: usize,
    pub name: Option<String>,
    pub buffer_size: usize,
}

pub struct RttManager {
    rtt: Option<Rtt>,
}

impl RttManager {
    pub fn new() -> Self {
        Self {
            rtt: None,
        }
    }

    /// Attempt to attach to RTT on the target.
    pub fn attach(&mut self, core: &mut Core) -> Result<()> {
        match Rtt::attach(core) {
            Ok(rtt) => {
                self.rtt = Some(rtt);
                log::info!("Attached to RTT control block");
                Ok(())
            }
            Err(e) => {
                Err(anyhow::anyhow!("Failed to attach to RTT: {}", e))
            }
        }
    }

    pub fn is_attached(&self) -> bool {
        self.rtt.is_some()
    }

    pub fn get_up_channels(&mut self) -> Vec<RttChannelInfo> {
        let Some(rtt) = &mut self.rtt else { return Vec::new(); };
        rtt.up_channels().iter().map(|c| RttChannelInfo {
            number: c.number(),
            name: c.name().map(|s| s.to_string()),
            buffer_size: c.buffer_size(),
        }).collect()
    }

    pub fn get_down_channels(&mut self) -> Vec<RttChannelInfo> {
        let Some(rtt) = &mut self.rtt else { return Vec::new(); };
        rtt.down_channels().iter().map(|c| RttChannelInfo {
            number: c.number(),
            name: c.name().map(|s| s.to_string()),
            buffer_size: c.buffer_size(),
        }).collect()
    }

    /// Read data from an up channel. Returns the data read.
    pub fn read_channel(&mut self, core: &mut Core, channel_number: usize) -> Result<Vec<u8>> {
        let rtt = self.rtt.as_mut().context("RTT not attached")?;
        let mut channel = rtt.up_channel(channel_number)
            .context(format!("Up channel {} not found", channel_number))?;

        let mut buffer = vec![0u8; channel.buffer_size()];
        let bytes_read = channel.read(core, &mut buffer)
            .context("Failed to read from RTT up channel")?;
        
        buffer.truncate(bytes_read);
        Ok(buffer)
    }

    /// Write data to a down channel.
    pub fn write_channel(&mut self, core: &mut Core, channel_number: usize, data: &[u8]) -> Result<usize> {
        let rtt = self.rtt.as_mut().context("RTT not attached")?;
        let mut channel = rtt.down_channel(channel_number)
            .context(format!("Down channel {} not found", channel_number))?;

        let bytes_written = channel.write(core, data)
            .context("Failed to write to RTT down channel")?;
        
        Ok(bytes_written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use probe_rs::{MemoryInterface, Error};

    struct MockMemory {
        data: std::collections::HashMap<u64, u8>,
    }

    impl MockMemory {
        fn new() -> Self {
            Self { data: std::collections::HashMap::new() }
        }
    }

    impl MemoryInterface for MockMemory {
        fn read_word_8(&mut self, address: u64) -> Result<u8, Error> {
            let mut b = [0u8; 1];
            self.read_8(address, &mut b)?;
            Ok(b[0])
        }
        fn read_word_16(&mut self, address: u64) -> Result<u16, Error> {
            let mut b = [0u8; 2];
            self.read_8(address, &mut b)?;
            Ok(u16::from_le_bytes(b))
        }
        fn read_word_32(&mut self, address: u64) -> Result<u32, Error> {
            let mut b = [0u8; 4];
            self.read_8(address, &mut b)?;
            Ok(u32::from_le_bytes(b))
        }
        fn read_word_64(&mut self, address: u64) -> Result<u64, Error> {
            let mut b = [0u8; 8];
            self.read_8(address, &mut b)?;
            Ok(u64::from_le_bytes(b))
        }
        fn write_word_8(&mut self, address: u64, data: u8) -> Result<(), Error> {
            self.write_8(address, &[data])
        }
        fn write_word_16(&mut self, address: u64, data: u16) -> Result<(), Error> {
            self.write_8(address, &data.to_le_bytes())
        }
        fn write_word_32(&mut self, address: u64, data: u32) -> Result<(), Error> {
            self.write_8(address, &data.to_le_bytes())
        }
        fn write_word_64(&mut self, address: u64, data: u64) -> Result<(), Error> {
            self.write_8(address, &data.to_le_bytes())
        }
        fn read_8(&mut self, address: u64, data: &mut [u8]) -> Result<(), Error> {
            for (i, byte) in data.iter_mut().enumerate() {
                *byte = *self.data.get(&(address + i as u64)).unwrap_or(&0);
            }
            Ok(())
        }
        fn write_8(&mut self, address: u64, data: &[u8]) -> Result<(), Error> {
            for (i, &byte) in data.iter().enumerate() {
                self.data.insert(address + i as u64, byte);
            }
            Ok(())
        }
        fn read_16(&mut self, address: u64, data: &mut [u16]) -> Result<(), Error> {
            for (i, word) in data.iter_mut().enumerate() {
                *word = self.read_word_16(address + (i * 2) as u64)?;
            }
            Ok(())
        }
        fn write_16(&mut self, address: u64, data: &[u16]) -> Result<(), Error> {
            for (i, &word) in data.iter().enumerate() {
                self.write_word_16(address + (i * 2) as u64, word)?;
            }
            Ok(())
        }
        fn read_32(&mut self, address: u64, data: &mut [u32]) -> Result<(), Error> {
            for (i, word) in data.iter_mut().enumerate() {
                *word = self.read_word_32(address + (i * 4) as u64)?;
            }
            Ok(())
        }
        fn write_32(&mut self, address: u64, data: &[u32]) -> Result<(), Error> {
            for (i, &word) in data.iter().enumerate() {
                self.write_word_32(address + (i * 4) as u64, word)?;
            }
            Ok(())
        }
        fn read_64(&mut self, address: u64, data: &mut [u64]) -> Result<(), Error> {
            for (i, word) in data.iter_mut().enumerate() {
                *word = self.read_word_64(address + (i * 8) as u64)?;
            }
            Ok(())
        }
        fn write_64(&mut self, address: u64, data: &[u64]) -> Result<(), Error> {
            for (i, &word) in data.iter().enumerate() {
                self.write_word_64(address + (i * 8) as u64, word)?;
            }
            Ok(())
        }
        fn flush(&mut self) -> Result<(), Error> { Ok(()) }
        fn supports_native_64bit_access(&mut self) -> bool { false }
        fn supports_8bit_transfers(&self) -> Result<bool, Error> { Ok(true) }
    }

    #[test]
    fn test_rtt_manager_initial_state() {
        let mgr = RttManager::new();
        assert!(!mgr.is_attached());
    }

/*
    #[test]
    fn test_rtt_attach_not_found() {
        let mut mock = MockMemory::new();
        let mut mgr = RttManager::new();
        // Should fail because signature is not present
        assert!(mgr.attach(&mut mock).is_err());
        assert!(!mgr.is_attached());
    }
*/
}
