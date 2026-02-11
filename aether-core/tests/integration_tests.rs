//! Integration tests for the Aether Debugger.
//! This file tests the interaction between `aether-core` and external components.

use aether_core::{ProbeManager, ProbeType, TargetInfo};

#[test]
fn test_probe_enumeration_skeleton() {
    let manager = ProbeManager::new();
    let probes = manager.list_probes().expect("Should be able to list probes");

    // In a CI environment without hardware, this will likely be empty
    println!("Found {} probes", probes.len());
}

#[test]
fn test_probe_info_integrity() {
    // Verify that our internal ProbeInfo can be created correctly
    let info = aether_core::ProbeInfo {
        vendor_id: 0x0483,
        product_id: 0x374B,
        serial_number: Some("123456".to_string()),
        probe_type: ProbeType::StLink,
    };

    assert_eq!(info.vendor_id, 0x0483);
    assert!(info.name().contains("ST-Link"));
    assert!(info.name().contains("0483:374B"));
}

#[test]
fn test_target_info_interface() {
    let target = TargetInfo {
        name: "MockChip".to_string(),
        flash_size: 512,
        ram_size: 128,
        architecture: "Armv8-M".to_string(),
    };

    assert_eq!(target.name, "MockChip");
    assert!(target.flash_size > 0);
}

#[test]
#[ignore] // This test requires physical hardware or a simulator
fn test_real_probe_connection() {
    let manager = ProbeManager::new();
    if let Ok(probe) = manager.open_first_probe() {
        assert!(!probe.get_name().is_empty());
    }
}
