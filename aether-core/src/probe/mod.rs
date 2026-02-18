//! Probe management module.
//!
//! Handles probe enumeration, connection, and target detection.

use anyhow::{Context, Result};
use probe_rs::probe::list::Lister;
use probe_rs::probe::{DebugProbeInfo, Probe};
pub use probe_rs::probe::WireProtocol;

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
    /// Internal detection logic.
    fn detect_target_internal(
        &self,
        probe: Probe,
        target_name: &str,
        under_reset: bool,
    ) -> Result<(TargetInfo, probe_rs::Session)> {
        use probe_rs::Permissions;
        use probe_rs::config::MemoryRegion;

        use probe_rs::config::TargetSelector;
        
        let selector = if target_name.eq_ignore_ascii_case("auto") {
            TargetSelector::Auto
        } else {
            TargetSelector::Unspecified(target_name.to_string())
        };

        let session_res = if under_reset {
            probe.attach_under_reset(selector, Permissions::default())
        } else {
            probe.attach(selector.clone(), Permissions::default())
        };

        let session = match session_res {
            Ok(s) => s,
            Err(e) if !under_reset && target_name.eq_ignore_ascii_case("auto") => {
                log::warn!("Initial auto-attach failed: {}. Retrying under reset...", e);
                // We need to re-open the probe because the previous attach attempt might have consumed it or left it in a bad state
                // However, detect_target_internal is called with a moved Probe.
                // This suggests we should probably handle the retry at a higher level (in connect)
                // or change the signature.
                // For now, let's try to reuse the probe if possible (probe-rs allows this sometimes)
                // but usually you need to re-open.
                return Err(e).context(format!("Failed to attach to target ({})", target_name));
            }
            Err(e) => return Err(e).context(format!("Failed to attach to target ({})", target_name)),
        };

        let target = session.target();

        // Extract flash/RAM info from the memory map
        let mut flash_size = 0;
        let mut ram_size = 0;

        for region in &target.memory_map {
            match region {
                MemoryRegion::Nvm(flash) => {
                    flash_size += flash.range.end - flash.range.start;
                }
                MemoryRegion::Ram(ram) => {
                    ram_size += ram.range.end - ram.range.start;
                }
                _ => {}
            }
        }

        let info = TargetInfo {
            name: target.name.clone(),
            flash_size,
            ram_size,
            architecture: format!("{:?}", target.architecture()),
        };

        Ok((info, session))
    }

    /// Connect to a target, optionally with protocol negotiation.
    pub fn connect(
        &self,
        probe_index: usize,
        target_name: &str,
        protocol: Option<WireProtocol>,
        under_reset: bool,
    ) -> Result<(TargetInfo, probe_rs::Session)> {
        let probes = self.lister.list_all();
        let probe_info = probes.get(probe_index).context("Probe index out of range")?;

        if let Some(proto) = protocol {
            // User specified protocol
            let mut probe = probe_info.open()?;
            probe.select_protocol(proto)?;
            
            match self.detect_target_internal(probe, target_name, under_reset) {
                Ok(res) => Ok(res),
                Err(e) if !under_reset && target_name.eq_ignore_ascii_case("auto") => {
                    log::warn!("Specified protocol ({:?}) attach failed. Retrying under reset...", proto);
                    let mut probe = probe_info.open()?;
                    probe.select_protocol(proto)?;
                    self.detect_target_internal(probe, target_name, true)
                }
                Err(e) => Err(e),
            }
        } else {
            // Automated negotiation: Try SWD (Normal -> Reset), then JTAG (Normal -> Reset)
            let protocols = [WireProtocol::Swd, WireProtocol::Jtag];
            let mut last_error = None;

            for &proto in &protocols {
                log::info!("Trying protocol: {:?}...", proto);
                
                // 1. Normal Attach
                let mut probe = match probe_info.open() {
                    Ok(p) => p,
                    Err(e) => {
                        log::warn!("Failed to open probe for {:?}: {}", proto, e);
                        continue;
                    }
                };
                let _ = probe.select_protocol(proto);
                let _ = probe.set_speed(1000); // Try lower speed for compatibility
                match self.detect_target_internal(probe, target_name, under_reset) {
                    Ok(res) => {
                        log::info!("Successfully attached with {:?} (Normal)", proto);
                        return Ok(res);
                    }
                    Err(e) => {
                        log::warn!("{:?} normal attach failed: {}", proto, e);
                        last_error = Some(e);
                    }
                }

                // 2. Fallback to Reset (if not already under reset and name is "auto")
                if !under_reset && target_name.eq_ignore_ascii_case("auto") {
                    log::info!("Trying {:?} under reset...", proto);
                    let mut probe = match probe_info.open() {
                        Ok(p) => p,
                        Err(e) => {
                            log::warn!("Failed to open probe for {:?} (Reset): {}", proto, e);
                            continue;
                        }
                    };
                    let _ = probe.select_protocol(proto);
                    let _ = probe.set_speed(1000);
                    match self.detect_target_internal(probe, target_name, true) {
                        Ok(res) => {
                            log::info!("Successfully attached with {:?} (Reset)", proto);
                            return Ok(res);
                        }
                        Err(e) => {
                            log::warn!("{:?} reset attach failed: {}", proto, e);
                            last_error = Some(e);
                        }
                    }
                }
            }

            // 3. Final Heuristic Fallback: Try common chips and Generic
            if target_name.eq_ignore_ascii_case("auto") {
                log::info!("Auto-detection exhausted. Trying heuristic fallbacks...");
                let heuristics = ["STM32L476RGTx", "STM32F407VGTx", "Cortex-M"];
                for chip in heuristics {
                    log::info!("Trying heuristic: {}...", chip);
                    let mut probe = match probe_info.open() {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    let _ = probe.select_protocol(WireProtocol::Swd);
                    let _ = probe.set_speed(1000);
                    if let Ok(res) = self.detect_target_internal(probe, chip, false) {
                        log::info!("Heuristic SUCCESS: Identified as {}", chip);
                        return Ok(res);
                    }
                }
            }

            Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Discovery failed"))).context("Zero-config attachment failed")
        }
    }

    /// Detect the target chip connected to the opened probe.
    /// If target_name is "auto", probe-rs will try to detect the chip automatically.
    /// Returns the TargetInfo and the active Session.
    pub fn detect_target(
        &self, 
        probe: Probe, 
        target_name: &str, 
        under_reset: bool,
    ) -> Result<(TargetInfo, probe_rs::Session)> {
        self.detect_target_internal(probe, target_name, under_reset)
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
