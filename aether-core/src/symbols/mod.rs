use anyhow::Result;
use gimli::{Abbreviations, AttributeValue, DebugStr, EndianSlice, RunTimeEndian, UnitOffset};
use object::{Object, ObjectSection, ObjectSymbol};
use probe_rs_debug::DebugInfo;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// Information about a source code location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub file: PathBuf,
    pub line: u32,
    pub column: Option<u32>,
    pub function: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: String,
    pub value_formatted_string: String,
    pub kind: String, // "Enum", "Struct", "Primitive", "Array", "Pointer"
    pub members: Option<Vec<TypeInfo>>,
    pub address: Option<u64>,
}

/// Manager for handling debugging symbols (DWARF).
pub struct SymbolManager {
    debug_info: Option<DebugInfo>,
    elf_data: Option<Vec<u8>>,
}

impl SymbolManager {
    pub fn new() -> Self {
        Self { debug_info: None, elf_data: None }
    }

    /// Load symbols from an ELF file.
    pub fn load_elf(&mut self, path: &Path) -> Result<()> {
        let data = std::fs::read(path)?;
        let debug_info = DebugInfo::from_file(path)
            .map_err(|e| anyhow::anyhow!("Failed to parse ELF/DWARF with probe-rs: {:?}", e))?;

        self.debug_info = Some(debug_info);
        self.elf_data = Some(data);
        log::info!("Loaded symbols from {}", path.display());
        Ok(())
    }

    /// Map a program counter address to a source location.
    pub fn lookup(&self, address: u64) -> Option<SourceInfo> {
        let debug_info = self.debug_info.as_ref()?;

        // probe-rs-debug 0.31 API
        let location = debug_info.get_source_location(address)?;

        // Convert TypedPathBuf to PathBuf via string representation
        let path_str = location.path.to_string_lossy().to_string();
        let file = PathBuf::from(path_str);

        Some(SourceInfo {
            file,
            line: location.line.map(|l| l as u32).unwrap_or(0),
            column: location.column.map(|c| match c {
                probe_rs_debug::ColumnType::Column(val) => val as u32,
                probe_rs_debug::ColumnType::LeftEdge => 0,
            }),
            function: None, // Function name not easily accessible without unwinding
        })
    }

    pub fn has_symbols(&self) -> bool {
        self.debug_info.is_some()
    }

    pub fn debug_info(&self) -> Option<&DebugInfo> {
        self.debug_info.as_ref()
    }

    pub fn elf_data(&self) -> Option<&[u8]> {
        self.elf_data.as_deref()
    }

    /// Map a source location to a program counter address.
    pub fn get_address(&self, target_file: &Path, target_line: u32) -> Option<u64> {
        let data = self.elf_data.as_ref()?;
        let obj = object::File::parse(&**data).ok()?;
        let endian =
            if obj.is_little_endian() { RunTimeEndian::Little } else { RunTimeEndian::Big };

        // Helper to load sections ensuring we don't drop data prematurely
        // We iterate specifically here rather than using Dwarf::load heavily

        // 1. Get .debug_line section
        let debug_line_section = obj.section_by_name(".debug_line")?;
        let debug_line_data = debug_line_section.uncompressed_data().ok()?;
        let debug_line = gimli::DebugLine::new(&debug_line_data, endian);

        // 2. Iterate units to find line programs.
        // PROPER WAY: We need to iterate .debug_info to get unit headers, which give us stmt_list offset in .debug_line
        let debug_info_section = obj.section_by_name(".debug_info")?;
        let debug_info_data = debug_info_section.uncompressed_data().ok()?;
        let debug_info = gimli::DebugInfo::new(&debug_info_data, endian);

        let debug_abbrev_section = obj.section_by_name(".debug_abbrev")?;
        let debug_abbrev_data = debug_abbrev_section.uncompressed_data().ok()?;
        let debug_abbrev = gimli::DebugAbbrev::new(&debug_abbrev_data, endian);

        let debug_str_section = obj
            .section_by_name(".debug_str")
            .and_then(|s| s.uncompressed_data().ok())
            .unwrap_or(Cow::Borrowed(&[]));
        let debug_str = gimli::DebugStr::new(&debug_str_section, endian);

        let mut iter = debug_info.units();
        while let Ok(Some(header)) = iter.next() {
            let abbrev = header.abbreviations(&debug_abbrev).ok()?;

            // Get DW_AT_stmt_list
            let mut tree = header.entries(&abbrev);
            let root = tree.next_dfs().ok()?.map(|(_, node)| node)?;
            let stmt_list = root.attr_value(gimli::DW_AT_stmt_list).ok()??;

            let offset = match stmt_list {
                gimli::AttributeValue::DebugLineRef(offset) => offset,
                _ => continue,
            };

            let program = debug_line.program(offset, header.address_size(), None, None).ok()?;
            let header = program.header();

            // Check files matches
            let mut file_idx = None;
            let file_index_base = if header.version() < 5 { 1 } else { 0 };

            for (i, file_entry) in header.file_names().iter().enumerate() {
                let name = file_entry.path_name();
                let name_str_opt = match name {
                    AttributeValue::String(slice) => String::from_utf8_lossy(&slice).to_string(),
                    AttributeValue::DebugStrRef(offset) => {
                        if let Ok(s) = debug_str.get_str(offset) {
                            String::from_utf8_lossy(&s).to_string()
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                };

                // Approximate matching: check if component ends with target_file
                if target_file.to_string_lossy().ends_with(&name_str_opt) {
                    file_idx = Some((i as u64) + file_index_base);
                    break;
                }
            }

            if let Some(idx) = file_idx {
                // Found file, iterate rows
                let mut rows = program.rows();
                while let Ok(Some((_, row))) = rows.next_row() {
                    if row.is_stmt() && row.file_index() == idx {
                        if let Some(line) = row.line() {
                            if line.get() == target_line as u64 {
                                return Some(row.address());
                            }
                        }
                    }
                }
            }
        }

        None
    }

    pub fn lookup_symbol(&self, name: &str) -> Option<u64> {
        let data = self.elf_data.as_ref()?;
        let obj = object::File::parse(&**data).ok()?;

        for symbol in obj.symbols() {
            if let Ok(sym_name) = symbol.name() {
                if sym_name == name {
                    return Some(symbol.address());
                }
            }
        }
        None
    }

    /// Resolve a raw value into a high-level TypeInfo using DWARF.
    pub fn resolve_variable(
        &self,
        core: &mut dyn probe_rs::MemoryInterface,
        name: &str,
        base_address: u64,
    ) -> Option<TypeInfo> {
        let elf_data = self.elf_data.as_ref()?;
        let obj = object::File::parse(&**elf_data).ok()?;
        let endian =
            if obj.is_little_endian() { RunTimeEndian::Little } else { RunTimeEndian::Big };

        let debug_info_section = obj.section_by_name(".debug_info")?;
        let debug_info_data = debug_info_section.uncompressed_data().ok()?;
        let debug_info = gimli::DebugInfo::new(&debug_info_data, endian);

        let debug_abbrev_section = obj.section_by_name(".debug_abbrev")?;
        let debug_abbrev_data = debug_abbrev_section.uncompressed_data().ok()?;
        let debug_abbrev = gimli::DebugAbbrev::new(&debug_abbrev_data, endian);

        let debug_str_section = obj
            .section_by_name(".debug_str")
            .and_then(|s| s.uncompressed_data().ok())
            .unwrap_or(Cow::Borrowed(&[]));
        let debug_str = gimli::DebugStr::new(&debug_str_section, endian);

        let mut units = debug_info.units();
        while let Ok(Some(header)) = units.next() {
            let abbrev = header.abbreviations(&debug_abbrev).ok()?;
            let mut entries = header.entries(&abbrev);

            while let Ok(Some((_, entry))) = entries.next_dfs() {
                if entry.tag() == gimli::DW_TAG_variable
                    || entry.tag() == gimli::DW_TAG_formal_parameter
                {
                    let entry_name =
                        entry.attr_value(gimli::DW_AT_name).ok().flatten().and_then(|attr| {
                            match attr {
                                AttributeValue::String(ref slice) => {
                                    Some(String::from_utf8_lossy(slice).to_string())
                                }
                                AttributeValue::DebugStrRef(offset) => debug_str
                                    .get_str(offset)
                                    .map(|s| String::from_utf8_lossy(&s).to_string())
                                    .ok(),
                                _ => None,
                            }
                        });

                    if let Some(en) = entry_name {
                        if en == name {
                            if let Ok(Some(AttributeValue::UnitRef(offset))) =
                                entry.attr_value(gimli::DW_AT_type)
                            {
                                if let Some(mut info) = self.resolve_type_from_offset(
                                    core,
                                    &header,
                                    &abbrev,
                                    &debug_str,
                                    offset,
                                    base_address,
                                    0,
                                ) {
                                    info.name = name.to_string();
                                    return Some(info);
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_type_from_offset(
        &self,
        core: &mut dyn probe_rs::MemoryInterface,
        header: &gimli::UnitHeader<EndianSlice<RunTimeEndian>>,
        abbrev: &Abbreviations,
        debug_str: &DebugStr<EndianSlice<RunTimeEndian>>,
        offset: UnitOffset,
        base_address: u64,
        depth: usize,
    ) -> Option<TypeInfo> {
        if depth > 10 {
            return None;
        }

        let mut entries = header.entries_at_offset(abbrev, offset).ok()?;
        let (_, entry) = entries.next_dfs().ok().flatten()?;

        let type_name = entry
            .attr_value(gimli::DW_AT_name)
            .ok()
            .flatten()
            .and_then(|attr| match attr {
                AttributeValue::String(ref slice) => {
                    Some(String::from_utf8_lossy(slice).to_string())
                }
                AttributeValue::DebugStrRef(off) => {
                    debug_str.get_str(off).map(|s| String::from_utf8_lossy(&s).to_string()).ok()
                }
                _ => None,
            })
            .unwrap_or_else(|| "unnamed".to_string());

        match entry.tag() {
            gimli::DW_TAG_base_type => {
                let size = entry
                    .attr_value(gimli::DW_AT_byte_size)
                    .ok()
                    .flatten()
                    .and_then(|v| match v {
                        AttributeValue::Udata(s) => Some(s),
                        _ => None,
                    })
                    .unwrap_or(4);

                let mut data = vec![0u8; size as usize];
                let value_str = if core.read(base_address, &mut data).is_ok() {
                    match size {
                        1 => format!("{}", data[0]),
                        2 => format!("{}", u16::from_le_bytes([data[0], data[1]])),
                        4 => {
                            format!("{}", u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
                        }
                        8 => format!(
                            "{}",
                            u64::from_le_bytes([
                                data[0], data[1], data[2], data[3], data[4], data[5], data[6],
                                data[7]
                            ])
                        ),
                        _ => format!("0x{:X}", base_address),
                    }
                } else {
                    "Error Reading".to_string()
                };

                Some(TypeInfo {
                    name: type_name,
                    value_formatted_string: value_str,
                    kind: "Primitive".to_string(),
                    members: None,
                    address: Some(base_address),
                })
            }
            gimli::DW_TAG_structure_type | gimli::DW_TAG_union_type => {
                let mut members = Vec::new();
                let mut children = header.entries_at_offset(abbrev, offset).ok()?;
                children.next_dfs().ok()?; // Skip self

                let mut current_depth = 0;
                while let Ok(Some((depth_delta, child))) = children.next_dfs() {
                    current_depth += depth_delta;
                    if current_depth <= 0 {
                        break;
                    }

                    if current_depth == 1 && child.tag() == gimli::DW_TAG_member {
                        let member_name = child
                            .attr_value(gimli::DW_AT_name)
                            .ok()
                            .flatten()
                            .and_then(|attr| match attr {
                                AttributeValue::String(ref slice) => {
                                    Some(String::from_utf8_lossy(slice).to_string())
                                }
                                AttributeValue::DebugStrRef(off) => debug_str
                                    .get_str(off)
                                    .map(|s| String::from_utf8_lossy(&s).to_string())
                                    .ok(),
                                _ => None,
                            })
                            .unwrap_or_else(|| "unnamed_member".to_string());

                        let member_offset = child
                            .attr_value(gimli::DW_AT_data_member_location)
                            .ok()
                            .flatten()
                            .and_then(|attr| match attr {
                                AttributeValue::Udata(off) => Some(off),
                                _ => None,
                            })
                            .unwrap_or(0);

                        if let Ok(Some(AttributeValue::UnitRef(type_off))) =
                            child.attr_value(gimli::DW_AT_type)
                        {
                            if let Some(mut member_info) = self.resolve_type_from_offset(
                                core,
                                header,
                                abbrev,
                                debug_str,
                                type_off,
                                base_address + member_offset,
                                depth + 1,
                            ) {
                                member_info.name = member_name;
                                members.push(member_info);
                            }
                        }
                    } else if current_depth == 1 && child.tag() == gimli::DW_TAG_variant_part {
                        // Handle enum variants (Option/Result)
                        // This is a simplification: we'll try to find the active variant
                        let mut variant_children =
                            header.entries_at_offset(abbrev, child.offset()).ok()?;
                        variant_children.next_dfs().ok()?; // Skip variant_part

                        let mut v_depth = 0;
                        while let Ok(Some((v_delta, v_child))) = variant_children.next_dfs() {
                            v_depth += v_delta;
                            if v_depth <= 0 {
                                break;
                            }

                            if v_depth == 1 && v_child.tag() == gimli::DW_TAG_variant {
                                let discr_val = v_child
                                    .attr_value(gimli::DW_AT_discr_value)
                                    .ok()
                                    .flatten()
                                    .and_then(|v| match v {
                                        AttributeValue::Data1(d) => Some(d as u64),
                                        AttributeValue::Data2(d) => Some(d as u64),
                                        AttributeValue::Data4(d) => Some(d as u64),
                                        AttributeValue::Data8(d) => Some(d),
                                        AttributeValue::Sdata(d) => Some(d as u64),
                                        AttributeValue::Udata(d) => Some(d),
                                        _ => None,
                                    })
                                    .unwrap_or(0);

                                // For now, we'll collect all variants as pseudo-members
                                // In a better implementation, we'd read the discriminant from memory
                                let mut v_entries =
                                    header.entries_at_offset(abbrev, v_child.offset()).ok()?;
                                v_entries.next_dfs().ok()?;
                                let mut vd = 0;
                                while let Ok(Some((vd_delta, vd_child))) = v_entries.next_dfs() {
                                    vd += vd_delta;
                                    if vd <= 0 {
                                        break;
                                    }
                                    if vd == 1 && vd_child.tag() == gimli::DW_TAG_member {
                                        let v_member_name = vd_child
                                            .attr_value(gimli::DW_AT_name)
                                            .ok()
                                            .flatten()
                                            .and_then(|attr| match attr {
                                                AttributeValue::String(ref slice) => {
                                                    Some(String::from_utf8_lossy(slice).to_string())
                                                }
                                                AttributeValue::DebugStrRef(off) => debug_str
                                                    .get_str(off)
                                                    .map(|s| {
                                                        String::from_utf8_lossy(&s).to_string()
                                                    })
                                                    .ok(),
                                                _ => None,
                                            })
                                            .unwrap_or_else(|| format!("Variant_{}", discr_val));

                                        if let Ok(Some(AttributeValue::UnitRef(type_off))) =
                                            vd_child.attr_value(gimli::DW_AT_type)
                                        {
                                            if let Some(mut m_info) = self.resolve_type_from_offset(
                                                core,
                                                header,
                                                abbrev,
                                                debug_str,
                                                type_off,
                                                base_address,
                                                depth + 1,
                                            ) {
                                                m_info.name = v_member_name;
                                                members.push(m_info);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if type_name.starts_with("Vec<") || type_name.starts_with("alloc::vec::Vec<") {
                    if let Some(len_member) = members.iter().find(|m| m.name == "len") {
                        if let Some(addr) = len_member.address {
                            let mut data = [0u8; 8];
                            if core.read(addr, &mut data).is_ok() {
                                let len = u64::from_le_bytes(data); // Assume LE for now, should use endian
                                return Some(TypeInfo {
                                    name: type_name.clone(),
                                    value_formatted_string: format!("Vec (len: {})", len),
                                    kind: "Array".to_string(),
                                    members: Some(members.to_vec()),
                                    address: Some(base_address),
                                });
                            }
                        }
                    }
                }

                Some(TypeInfo {
                    name: type_name.clone(),
                    value_formatted_string: type_name.clone(),
                    kind: if entry.tag() == gimli::DW_TAG_union_type {
                        "Union".to_string()
                    } else {
                        "Struct".to_string()
                    },
                    members: if members.is_empty() { None } else { Some(members) },
                    address: Some(base_address),
                })
            }
            gimli::DW_TAG_pointer_type => Some(TypeInfo {
                name: format!("*{}", type_name),
                value_formatted_string: format!("0x{:X}", base_address),
                kind: "Pointer".to_string(),
                members: None,
                address: Some(base_address),
            }),
            gimli::DW_TAG_enumeration_type => {
                // Handle Option/Result discriminants if they look like it
                Some(TypeInfo {
                    name: type_name.clone(),
                    value_formatted_string: type_name.clone(),
                    kind: "Enum".to_string(),
                    members: None,
                    address: Some(base_address),
                })
            }
            gimli::DW_TAG_const_type | gimli::DW_TAG_volatile_type | gimli::DW_TAG_typedef => {
                if let Ok(Some(AttributeValue::UnitRef(type_off))) =
                    entry.attr_value(gimli::DW_AT_type)
                {
                    self.resolve_type_from_offset(
                        core,
                        header,
                        abbrev,
                        debug_str,
                        type_off,
                        base_address,
                        depth,
                    )
                } else {
                    None
                }
            }
            _ => Some(TypeInfo {
                name: type_name,
                value_formatted_string: format!("0x{:X}", base_address),
                kind: "Primitive".to_string(),
                members: None,
                address: Some(base_address),
            }),
        }
    }
}

impl Default for SymbolManager {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_manager_initial_state() {
        let mgr = SymbolManager::new();
        assert!(!mgr.has_symbols());
        assert!(mgr.elf_data().is_none());
    }

    #[test]
    fn test_symbol_lookup_no_symbols() {
        let mgr = SymbolManager::new();
        assert!(mgr.lookup_symbol("main").is_none());
        assert!(mgr.lookup(0x1000).is_none());
    }
}
