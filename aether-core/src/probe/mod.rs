//! Probe management module.
//!
//! Handles probe enumeration, connection, and target detection.

use anyhow::{Context, Result};
use probe_rs::probe::list::Lister;
use probe_rs::probe::{DebugProbeInfo, Probe};

/// Information about an available debug probe.
#[derive(Debug, Clone)]
pub struct ProbeInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial_number: Option<String>,
    pub probe_type: ProbeType,
}

/// Type of debug probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeType {
    StLink,
    JLink,
    CmsisDap,
    Other,
}

impl From<&DebugProbeInfo> for ProbeInfo {
    fn from(info: &DebugProbeInfo) -> Self {
        let probe_type = match (info.vendor_id, info.product_id) {
            (0x0483, _) => ProbeType::StLink,
            (0x1366, _) => ProbeType::JLink,
            (0x0D28, _) => ProbeType::CmsisDap,
            _ => ProbeType::Other,
        };

        ProbeInfo {
            vendor_id: info.vendor_id,
            product_id: info.product_id,
            serial_number: info.serial_number.clone(),
            probe_type,
        }
    }
}

impl ProbeInfo {
    /// Get a human-readable name for this probe.
    pub fn name(&self) -> String {
        match self.probe_type {
            ProbeType::StLink => {
                format!("ST-Link ({:04X}:{:04X})", self.vendor_id, self.product_id)
            }
            ProbeType::JLink => format!("J-Link ({:04X}:{:04X})", self.vendor_id, self.product_id),
            ProbeType::CmsisDap => {
                format!("CMSIS-DAP ({:04X}:{:04X})", self.vendor_id, self.product_id)
            }
            ProbeType::Other => format!("Unknown ({:04X}:{:04X})", self.vendor_id, self.product_id),
        }
    }
}

/// Probe manager for enumerating and connecting to debug probes.
pub struct ProbeManager {
    lister: Lister,
}

impl ProbeManager {
    /// Create a new probe manager.
    pub fn new() -> Self {
        Self { lister: Lister::new() }
    }

    /// List all available debug probes.
    pub fn list_probes(&self) -> Result<Vec<ProbeInfo>> {
        let probes = self.lister.list_all();
        Ok(probes.iter().map(ProbeInfo::from).collect())
    }

    /// Open a probe by index from the list.
    pub fn open_probe(&self, index: usize) -> Result<Probe> {
        let probes = self.lister.list_all();
        let probe_info = probes.get(index).context("Probe index out of range")?;

        probe_info.open().context("Failed to open probe")
    }

    /// Open the first available probe.
    pub fn open_first_probe(&self) -> Result<Probe> {
        let probes = self.lister.list_all();
        let probe_info = probes.into_iter().next().context("No debug probes found")?;

        probe_info.open().context("Failed to open probe")
    }
}

impl Default for ProbeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about the connected target chip.
#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub name: String,
    pub flash_size: u64,
    pub ram_size: u64,
    pub architecture: String,
}

impl ProbeManager {
    /// Detect the target chip connected to the opened probe using auto-detection.
    pub fn detect_target(&self, probe: Probe) -> Result<TargetInfo> {
        use probe_rs::config::MemoryRegion;
        use probe_rs::Permissions;

        // Use "auto" to let probe-rs try to detect the chip
        let session = probe
            .attach("auto", Permissions::default())
            .context("Failed to attach to target (auto-detect)")?;

        let target = session.target();

        // Extract flash/RAM info from the memory map
        let mut flash_size = 0;
        let mut ram_size = 0;

        for region in &target.memory_map {
            match region {
                MemoryRegion::Nvm(r) => flash_size += r.range.end - r.range.start,
                MemoryRegion::Ram(r) => ram_size += r.range.end - r.range.start,
                _ => {}
            }
        }

        Ok(TargetInfo {
            name: target.name.clone(),
            flash_size,
            ram_size,
            architecture: format!("{:?}", target.architecture()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_info_names() {
        let cases = vec![
            (ProbeType::StLink, 0x0483, 0x3748, "ST-Link (0483:3748)"),
            (ProbeType::JLink, 0x1366, 0x0101, "J-Link (1366:0101)"),
            (ProbeType::CmsisDap, 0x0D28, 0x0204, "CMSIS-DAP (0D28:0204)"),
            (ProbeType::Other, 0xFFFF, 0xFFFF, "Unknown (FFFF:FFFF)"),
        ];

        for (pt, vid, pid, expected_name) in cases {
            let info =
                ProbeInfo { vendor_id: vid, product_id: pid, serial_number: None, probe_type: pt };
            assert_eq!(info.name(), expected_name);
        }
    }

    #[test]
    fn test_probe_info_with_serial() {
        let info = ProbeInfo {
            vendor_id: 0x0483,
            product_id: 0x3748,
            serial_number: Some("ABC123".to_string()),
            probe_type: ProbeType::StLink,
        };
        // The current name() implementation doesn't include serial, but we verify it's stored
        assert_eq!(info.serial_number, Some("ABC123".to_string()));
    }

    #[test]
    fn test_probe_type_conversion() {
        // Simple sanity check for the enum
        let st_link = ProbeType::StLink;
        assert_eq!(st_link, ProbeType::StLink);
    }

    #[test]
    fn test_target_info_struct() {
        let info = TargetInfo {
            name: "STM32F407VGTx".to_string(),
            flash_size: 1024 * 1024,
            ram_size: 192 * 1024,
            architecture: "Armv7em".to_string(),
        };
        assert_eq!(info.name, "STM32F407VGTx");
        assert_eq!(info.flash_size, 1048576);
    }

    #[test]
    fn test_target_info_edge_cases() {
        let info = TargetInfo {
            name: "EmptyChip".to_string(),
            flash_size: 0,
            ram_size: 0,
            architecture: "Unknown".to_string(),
        };
        assert_eq!(info.flash_size, 0);
        assert_eq!(info.ram_size, 0);
    }

    #[test]
    fn test_probe_manager_default() {
        let _ = ProbeManager::default();
    }
}
