#[cfg(not(feature = "hardware"))]
use crate::probe_rs::Core;
use anyhow::{Context as _, Result};
#[cfg(feature = "hardware")]
use probe_rs::rtt::Rtt;
#[cfg(feature = "hardware")]
use probe_rs::Core;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RttChannelInfo {
    pub number: usize,
    pub name: Option<String>,
    pub buffer_size: usize,
}

pub struct RttManager {
    #[cfg(feature = "hardware")]
    rtt: Option<Rtt>,
    #[cfg(not(feature = "hardware"))]
    rtt: Option<()>,
}

impl Default for RttManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RttManager {
    pub fn new() -> Self {
        Self { rtt: None }
    }

    /// Attempt to attach to RTT on the target.
    pub fn attach(&mut self, core: &mut Core) -> Result<()> {
        #[cfg(feature = "hardware")]
        {
            match Rtt::attach(core) {
                Ok(rtt) => {
                    self.rtt = Some(rtt);
                    log::info!("Attached to RTT control block");
                    Ok(())
                }
                Err(e) => Err(anyhow::anyhow!("Failed to attach to RTT: {}", e)),
            }
        }
        #[cfg(not(feature = "hardware"))]
        {
            let _ = core;
            Ok(())
        }
    }

    pub fn is_attached(&self) -> bool {
        self.rtt.is_some()
    }

    pub fn get_up_channels(&mut self) -> Vec<RttChannelInfo> {
        #[cfg(feature = "hardware")]
        {
            let Some(rtt) = &mut self.rtt else {
                return Vec::new();
            };
            rtt.up_channels()
                .iter()
                .map(|c| RttChannelInfo {
                    number: c.number(),
                    name: c.name().map(|s| s.to_string()),
                    buffer_size: c.buffer_size(),
                })
                .collect()
        }
        #[cfg(not(feature = "hardware"))]
        Vec::new()
    }

    pub fn get_down_channels(&mut self) -> Vec<RttChannelInfo> {
        #[cfg(feature = "hardware")]
        {
            let Some(rtt) = &mut self.rtt else {
                return Vec::new();
            };
            rtt.down_channels()
                .iter()
                .map(|c| RttChannelInfo {
                    number: c.number(),
                    name: c.name().map(|s| s.to_string()),
                    buffer_size: c.buffer_size(),
                })
                .collect()
        }
        #[cfg(not(feature = "hardware"))]
        Vec::new()
    }

    /// Read data from an up channel. Returns the data read.
    pub fn read_channel(&mut self, core: &mut Core, channel_number: usize) -> Result<Vec<u8>> {
        #[cfg(feature = "hardware")]
        {
            let rtt = self.rtt.as_mut().context("RTT not attached")?;
            let channel = rtt
                .up_channel(channel_number)
                .context(format!("Up channel {} not found", channel_number))?;

            let mut buffer = vec![0u8; channel.buffer_size()];
            let bytes_read =
                channel.read(core, &mut buffer).context("Failed to read from RTT up channel")?;

            buffer.truncate(bytes_read);
            Ok(buffer)
        }
        #[cfg(not(feature = "hardware"))]
        {
            let _ = (core, channel_number);
            Ok(Vec::new())
        }
    }

    /// Write data to a down channel.
    pub fn write_channel(
        &mut self,
        core: &mut Core,
        channel_number: usize,
        data: &[u8],
    ) -> Result<usize> {
        #[cfg(feature = "hardware")]
        {
            let rtt = self.rtt.as_mut().context("RTT not attached")?;
            let channel = rtt
                .down_channel(channel_number)
                .context(format!("Down channel {} not found", channel_number))?;

            let bytes_written =
                channel.write(core, data).context("Failed to write to RTT down channel")?;

            Ok(bytes_written)
        }
        #[cfg(not(feature = "hardware"))]
        {
            let _ = (core, channel_number, data);
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtt_manager_initial_state() {
        let mgr = RttManager::new();
        assert!(!mgr.is_attached());
    }
}
