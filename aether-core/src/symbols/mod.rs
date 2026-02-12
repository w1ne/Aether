use anyhow::Result;
use std::path::{Path, PathBuf};
use probe_rs_debug::DebugInfo;
use serde::{Serialize, Deserialize};
use object::{Object, ObjectSection, ObjectSymbol};
use gimli::{RunTimeEndian, AttributeValue};
use std::borrow::Cow;

/// Information about a source code location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub file: PathBuf,
    pub line: u32,
    pub column: Option<u32>,
    pub function: Option<String>,
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
        let endian = if obj.is_little_endian() { RunTimeEndian::Little } else { RunTimeEndian::Big };

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

        let debug_str_section = obj.section_by_name(".debug_str").map(|s| s.uncompressed_data().ok()).flatten().unwrap_or(Cow::Borrowed(&[]));
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

    /// Lookup a symbol address by name from the ELF symbol table.
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
