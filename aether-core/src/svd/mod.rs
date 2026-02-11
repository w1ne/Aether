//! SVD parsing and register decoding module.

use anyhow::{Context as _, Result};
use svd_parser as svd;
use svd_rs as rs;
use std::fs;
use std::path::Path;
use probe_rs::MemoryInterface;

/// Manager for SVD operations.
#[derive(Default)]
pub struct SvdManager {
    pub device: Option<rs::Device>,
}

impl SvdManager {
    pub fn new() -> Self {
        Self { device: None }
    }

    /// Load an SVD file from disk.
    pub fn load_svd<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let xml = fs::read_to_string(path).context("Failed to read SVD file")?;
        let device = svd::parse(&xml).context("Failed to parse SVD XML")?;
        self.device = Some(device);
        Ok(())
    }

    /// Get list of peripheral names.
    pub fn list_peripherals(&self) -> Vec<String> {
        self.device.as_ref()
            .map(|d| d.peripherals.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default()
    }

    /// Find a peripheral by name.
    pub fn get_peripheral(&self, name: &str) -> Option<&rs::Peripheral> {
        self.device.as_ref()?
            .peripherals.iter()
            .find(|p| p.name == name)
    }

    /// Get detailed peripheral info.
    pub fn get_peripherals_info(&self) -> Vec<PeripheralInfo> {
        self.device.as_ref()
            .map(|d| {
                d.peripherals.iter().map(|p| PeripheralInfo {
                    name: p.name.clone(),
                    base_address: p.base_address,
                    description: p.description.clone(),
                }).collect()
            })
            .unwrap_or_default()
    }

    /// Get detailed registers for a peripheral.
    pub fn get_registers_info(&self, peripheral_name: &str) -> Result<Vec<RegisterInfo>> {
        let p = self.get_peripheral(peripheral_name)
            .context(format!("Peripheral {} not found", peripheral_name))?;

        let registers = match &p.registers {
             Some(regs) => regs,
             None => return Ok(Vec::new()),
        };

        let mut infos = Vec::new();
        for node in registers {
            // Handle clusters if needed in the future
            if let rs::RegisterCluster::Register(r) = node {
                let mut fields = Vec::new();
                if let Some(f_list) = &r.fields {
                    for f in f_list {
                        fields.push(FieldInfo {
                            name: f.name.clone(),
                            description: f.description.clone(),
                            bit_offset: f.bit_offset(),
                            bit_width: f.bit_width(),
                        });
                    }
                }

                infos.push(RegisterInfo {
                    name: r.name.clone(),
                    address_offset: r.address_offset,
                    description: r.description.clone(),
                    size: r.properties.size.unwrap_or(32),
                    fields,
                    value: None,
                });
            }
        }

        Ok(infos)
    }

    /// Read values for all registers in a peripheral.
    pub fn read_peripheral_values(
        &self,
        peripheral_name: &str,
        core: &mut probe_rs::Core,
    ) -> Result<Vec<RegisterInfo>> {
        let p = self.get_peripheral(peripheral_name)
            .context(format!("Peripheral {} not found", peripheral_name))?;

        let mut regs = self.get_registers_info(peripheral_name)?;
        let base_addr = p.base_address;

        for reg in &mut regs {
            let addr = base_addr + reg.address_offset as u64;
            let val = match reg.size {
                8 => core.read_word_8(addr).map(|v| v as u64),
                16 => core.read_word_16(addr).map(|v| v as u64),
                32 => core.read_word_32(addr).map(|v| v as u64),
                64 => core.read_word_64(addr),
                _ => core.read_word_32(addr).map(|v| v as u64),
            };

            if let Ok(v) = val {
                reg.value = Some(v);
            }
        }

        Ok(regs)
    }

    /// Write a new value to a specific field in a peripheral register.
    pub fn write_peripheral_field(
        &self,
        core: &mut probe_rs::Core,
        peripheral_name: &str,
        register_name: &str,
        field_name: &str,
        new_field_value: u64,
    ) -> Result<()> {
        let p = self.get_peripheral(peripheral_name)
            .context(format!("Peripheral {} not found", peripheral_name))?;

        let regs = self.get_registers_info(peripheral_name)?;
        let reg = regs.iter().find(|r| r.name == register_name)
            .context(format!("Register {} not found in peripheral {}", register_name, peripheral_name))?;

        let field = reg.fields.iter().find(|f| f.name == field_name)
            .context(format!("Field {} not found in register {}", field_name, register_name))?;

        let addr = p.base_address + reg.address_offset as u64;

        // 1. Read current value
        let current_val = match reg.size {
            8 => core.read_word_8(addr).map(|v| v as u64),
            16 => core.read_word_16(addr).map(|v| v as u64),
            32 => core.read_word_32(addr).map(|v| v as u64),
            64 => core.read_word_64(addr),
            _ => core.read_word_32(addr).map(|v| v as u64),
        }.context("Failed to read register for write-modify-read")?;

        // 2. Modify field
        let mask = ((1u64 << field.bit_width) - 1) << field.bit_offset;
        let masked_new_val = (new_field_value << field.bit_offset) & mask;
        let next_val = (current_val & !mask) | masked_new_val;

        // 3. Write back
        match reg.size {
            8 => core.write_word_8(addr, next_val as u8),
            16 => core.write_word_16(addr, next_val as u16),
            32 => core.write_word_32(addr, next_val as u32),
            64 => core.write_word_64(addr, next_val),
            _ => core.write_word_32(addr, next_val as u32),
        }.context("Failed to write register back")?;

        Ok(())
    }
}

/// Simplified representation for UI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeripheralInfo {
    pub name: String,
    pub base_address: u64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegisterInfo {
    pub name: String,
    pub address_offset: u32,
    pub description: Option<String>,
    pub size: u32,
    pub fields: Vec<FieldInfo>,
    pub value: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub description: Option<String>,
    pub bit_offset: u32,
    pub bit_width: u32,
}

impl FieldInfo {
    /// Decode the value of this field from a register value.
    pub fn decode(&self, reg_value: u64) -> u64 {
        let mask = ((1u64 << self.bit_width) - 1) << self.bit_offset;
        (reg_value & mask) >> self.bit_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_decoding() {
        let field = FieldInfo {
            name: "TEST".to_string(),
            description: None,
            bit_offset: 4,
            bit_width: 4,
        };

        // Reg value: 0x0000_00A0 -> Field [4..7] should be A (10)
        assert_eq!(field.decode(0x0000_00A0), 0xA);
        
        // Reg value: 0xFFFF_FFAF -> Field [4..7] should be A (10)
        assert_eq!(field.decode(0xFFFF_FFAF), 0xA);

        let multi_bit = FieldInfo {
            name: "MULTI".to_string(),
            description: None,
            bit_offset: 0,
            bit_width: 8,
        };
        assert_eq!(multi_bit.decode(0x1234_5678), 0x78);
    }
}
