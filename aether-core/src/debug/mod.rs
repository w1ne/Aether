//! Debug control module.
//!
//! Handles core debug operations: halt, resume, step, and register access.

pub mod breakpoint;

pub use breakpoint::BreakpointManager;

use anyhow::{Context, Result};
#[cfg(feature = "hardware")]
use probe_rs::{Core, CoreInformation, CoreStatus};
#[cfg(not(feature = "hardware"))]
use crate::probe_rs::{Core, CoreInformation, CoreStatus};
use std::time::Duration;

/// Manager for debug operations.
pub struct DebugManager;

impl DebugManager {
    pub fn new() -> Self {
        Self
    }

    /// Halt the core.
    pub fn halt(&self, core: &mut Core) -> Result<CoreInformation> {
        core.halt(Duration::from_millis(100)).context("Failed to halt core")
    }

    /// Resume the core.
    pub fn resume(&self, core: &mut Core) -> Result<()> {
        core.run().context("Failed to resume core")
    }

    /// Step the core by one instruction.
    pub fn step(&self, core: &mut Core) -> Result<CoreInformation> {
        core.step().context("Failed to step core")
    }

    /// Get the current status of the core.
    pub fn status(&self, core: &mut Core) -> Result<CoreStatus> {
        core.status().context("Failed to get core status")
    }

    /// Read a core register.
    pub fn read_core_reg(&self, core: &mut Core, address: u16) -> Result<u64> {
        #[cfg(feature = "hardware")]
        let val = core.read_core_reg(address).context("Failed to read core register")?;
        #[cfg(not(feature = "hardware"))]
        let val = core.read_core_reg(u32::from(address)).context("Failed to read core register")?;
        #[cfg(feature = "hardware")]
        return Ok(match val {
            probe_rs::RegisterValue::U32(v) => v as u64,
            probe_rs::RegisterValue::U64(v) => v,
            probe_rs::RegisterValue::U128(v) => v as u64,
        });
        #[cfg(not(feature = "hardware"))]
        return Ok(match val {
            crate::probe_rs::RegisterValue::U32(v) => v as u64,
            crate::probe_rs::RegisterValue::U64(v) => v,
            crate::probe_rs::RegisterValue::U128(v) => v as u64,
        });
    }

    /// Write a core register.
    pub fn write_core_reg(&self, core: &mut Core, address: u16, value: u64) -> Result<()> {
        #[cfg(feature = "hardware")]
        let reg_val = probe_rs::RegisterValue::U64(value);
        #[cfg(not(feature = "hardware"))]
        let reg_val = crate::probe_rs::RegisterValue::U64(value);

        #[cfg(feature = "hardware")]
        return core.write_core_reg(address, reg_val).context("Failed to write core register");
        #[cfg(not(feature = "hardware"))]
        return core.write_core_reg(u32::from(address), reg_val).context("Failed to write core register");
    }
}

impl Default for DebugManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_manager_creation() {
        let _mgr = DebugManager::new();
        let _default_mgr = DebugManager::new();
    }
}
