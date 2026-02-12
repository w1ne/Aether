use probe_rs::Session;
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceConfig {
    pub core_frequency: u32,
    pub trace_frequency: u32, // SWO baud rate
    pub itm_ports: Vec<u32>,
}

pub struct TraceManager {
    enabled: bool,
    config: Option<TraceConfig>,
}

impl TraceManager {
    pub fn new() -> Self {
        Self {
            enabled: false,
            config: None,
        }
    }

    pub fn enable(&mut self, _session: &mut Session, config: TraceConfig) -> Result<()> {
        // Configure SWV (Serial Wire Viewer)
        // This requires probe-rs specific API.
        // session.setup_swv(...) exists in recent probe-rs versions.
        
        // Use a generic approach if setup_swv is not stable or available in 0.31
        // But 0.31 definitely has it.
        
        // Note: We need to import Architecture to construct SwayConfig correctly if needed.
        // For now, this is a placeholder to be filled with actual SWV setup.
        
        self.config = Some(config);
        self.enabled = true;
        Ok(())
    }
    
    pub fn read_data(&mut self, _session: &mut Session) -> Result<Vec<u8>> {
        if !self.enabled {
            return Ok(Vec::new());
        }
        
        // session.read_swv() returns Result<Vec<u8>>
        // We'll wrap it here.
        // let data = session.read_swv()?;
        // Ok(data)
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_manager_initial_state() {
        let mgr = TraceManager::new();
        assert!(!mgr.enabled);
        assert!(mgr.config.is_none());
    }

    #[test]
    fn test_trace_config_serialization() {
        let config = TraceConfig {
            core_frequency: 168_000_000,
            trace_frequency: 2_000_000,
            itm_ports: vec![0],
        };
        let json = serde_json::to_string(&config).unwrap();
        let decoded: TraceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, decoded);
    }
}
