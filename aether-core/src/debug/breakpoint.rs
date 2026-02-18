//! Breakpoint management module.

use anyhow::{Context, Result};
use probe_rs::Core;
use std::collections::HashSet;

/// Manager for core breakpoints.
pub struct BreakpointManager {
    breakpoints: HashSet<u64>,
}

impl BreakpointManager {
    pub fn new() -> Self {
        Self { breakpoints: HashSet::new() }
    }

    /// Set a hardware breakpoint at the given address.
    pub fn set_breakpoint(&mut self, core: &mut Core, address: u64) -> Result<()> {
        core.set_hw_breakpoint(address).context("Failed to set hardware breakpoint")?;
        self.breakpoints.insert(address);
        Ok(())
    }

    /// Clear a hardware breakpoint at the given address.
    pub fn clear_breakpoint(&mut self, core: &mut Core, address: u64) -> Result<()> {
        core.clear_hw_breakpoint(address).context("Failed to clear hardware breakpoint")?;
        self.breakpoints.remove(&address);
        Ok(())
    }

    /// Clear all breakpoints.
    pub fn clear_all(&mut self, core: &mut Core) -> Result<()> {
        for &addr in &self.breakpoints {
            core.clear_hw_breakpoint(addr)
                .context(format!("Failed to clear breakpoint @ 0x{:08X}", addr))?;
        }
        self.breakpoints.clear();
        Ok(())
    }

    /// Toggle a hardware breakpoint at the given address.
    pub fn toggle_breakpoint(&mut self, core: &mut Core, address: u64) -> Result<()> {
        if self.breakpoints.contains(&address) {
            self.clear_breakpoint(core, address)
        } else {
            self.set_breakpoint(core, address)
        }
    }

    /// List active breakpoint addresses.
    pub fn list(&self) -> Vec<u64> {
        self.breakpoints.iter().cloned().collect()
    }
}

impl Default for BreakpointManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breakpoint_manager_tracking() {
        let mgr = BreakpointManager::new();
        assert!(mgr.list().is_empty());

        // We can't easily test set_breakpoint without a Core mock,
        // but we can at least verify the manager creation.
    }
}
