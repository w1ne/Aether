use aether_core::symbols::SymbolManager;
use std::path::PathBuf;
use probe_rs::MemoryInterface;

struct MockMemory;
impl MemoryInterface for MockMemory {
    fn read(&mut self, _address: u64, data: &mut [u8]) -> Result<(), probe_rs::Error> {
        for b in data.iter_mut() { *b = 0; }
        Ok(())
    }
    fn read_word_32(&mut self, _address: u64) -> Result<u32, probe_rs::Error> { Ok(0) }
    fn read_word_64(&mut self, _address: u64) -> Result<u64, probe_rs::Error> { Ok(0) }
    fn read_8(&mut self, _address: u64, data: &mut [u8]) -> Result<(), probe_rs::Error> { self.read(_address, data) }
    fn read_16(&mut self, _address: u64, _data: &mut [u16]) -> Result<(), probe_rs::Error> { Ok(()) }
    fn read_32(&mut self, _address: u64, _data: &mut [u32]) -> Result<(), probe_rs::Error> { Ok(()) }
    fn read_64(&mut self, _address: u64, _data: &mut [u64]) -> Result<(), probe_rs::Error> { Ok(()) }
    fn write_word_32(&mut self, _address: u64, _data: u32) -> Result<(), probe_rs::Error> { Ok(()) }
    fn write_word_64(&mut self, _address: u64, _data: u64) -> Result<(), probe_rs::Error> { Ok(()) }
    fn write_8(&mut self, _address: u64, _data: &[u8]) -> Result<(), probe_rs::Error> { Ok(()) }
    fn write_16(&mut self, _address: u64, _data: &[u16]) -> Result<(), probe_rs::Error> { Ok(()) }
    fn write_32(&mut self, _address: u64, _data: &[u32]) -> Result<(), probe_rs::Error> { Ok(()) }
    fn write_64(&mut self, _address: u64, _data: &[u64]) -> Result<(), probe_rs::Error> { Ok(()) }
    fn flush(&mut self) -> Result<(), probe_rs::Error> { Ok(()) }
    fn supports_native_64bit_access(&mut self) -> bool { false }
    fn supports_8bit_transfers(&self) -> Result<bool, probe_rs::Error> { Ok(true) }
}

#[test]
fn test_dwarf_nested_resolution() {
    let mut core = MockMemory;
    let mut symbol_manager = SymbolManager::new();
    let mut elf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    elf_path.push("tests/fixtures/complex_types.elf");
    
    // Check if file exists first to give a better error message if compilation failed in a weird way
    assert!(elf_path.exists(), "ELF fixture not found at {:?}", elf_path);
    
    symbol_manager.load_elf(&elf_path).expect("Failed to load elf");
    
    // 1. Resolve MY_CONFIG (Global Struct)
    let addr = symbol_manager.lookup_symbol("MY_CONFIG").expect("Symbol 'MY_CONFIG' not found");
    let info = symbol_manager.resolve_variable(&mut core, "MY_CONFIG", addr).expect("Failed to resolve MY_CONFIG");
    
    assert_eq!(info.name, "MY_CONFIG");
    assert_eq!(info.kind, "Struct");
    
    let members = info.members.as_ref().expect("MY_CONFIG members missing");
    assert_eq!(members.len(), 3, "Expected 3 members in Config (enabled, threshold, nested)");
    
    // 2. Check 'enabled' (Base Type)
    let enabled = members.iter().find(|m| m.name == "enabled").expect("enabled member missing");
    assert_eq!(enabled.kind, "Primitive");
    
    // 3. Check 'nested' (Nested Struct)
    let nested = members.iter().find(|m| m.name == "nested").expect("nested member missing");
    assert_eq!(nested.kind, "Struct");
    
    let nested_members = nested.members.as_ref().expect("Nested members missing");
    assert_eq!(nested_members.len(), 3, "Expected 3 members in Nested (x, b, deep)");
    
    // 4. Check 'deep' inside 'nested' (Double Nested Struct)
    let deep = nested_members.iter().find(|m| m.name == "deep").expect("deep member missing");
    assert_eq!(deep.kind, "Struct");
    
    let deep_members = deep.members.as_ref().expect("Deep members missing");
    assert_eq!(deep_members.len(), 2, "Expected 2 members in Deep (a, b)");
    assert!(deep_members.iter().any(|m| m.name == "a"));
    assert!(deep_members.iter().any(|m| m.name == "b"));
}

#[test]
fn test_dwarf_recursion_safety() {
    // This is more of a safety check. Currently our resolver has a depth limit of 10.
    // If we ever had circular types (common in C/C++ with pointers, less so in plain Rust structs)
    // it should not stack overflow.
    
    // Note: Plain Rust structs cannot be recursive without indirection (Box, &, etc.)
    // which DWARF represents as Pointer tags, which our resolver doesn't recurse into yet.
}

#[test]
fn test_dwarf_rust_vec_resolution() {
    let mut core = MockMemory;
    let mut symbol_manager = SymbolManager::new();
    let mut elf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    elf_path.push("tests/fixtures/rust_types.elf");
    
    assert!(elf_path.exists(), "ELF fixture not found at {:?}", elf_path);
    symbol_manager.load_elf(&elf_path).expect("Failed to load elf");
    
    let addr = symbol_manager.lookup_symbol("G_V").expect("Symbol 'G_V' not found");
    let info = symbol_manager.resolve_variable(&mut core, "G_V", addr).expect("Failed to resolve G_V");
    
    assert!(info.name == "G_V");
    assert!(info.kind == "Array", "Expected kind Array for Vec, got {}", info.kind);
    assert!(info.value_formatted_string.contains("Vec (len: 0)"));
}

#[test]
fn test_dwarf_rust_option_resolution() {
    let mut core = MockMemory;
    let mut symbol_manager = SymbolManager::new();
    let mut elf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    elf_path.push("tests/fixtures/rust_types.elf");
    
    assert!(elf_path.exists(), "ELF fixture not found at {:?}", elf_path);
    symbol_manager.load_elf(&elf_path).expect("Failed to load elf");
    
    let addr = symbol_manager.lookup_symbol("G_O").expect("Symbol 'G_O' not found");
    let info = symbol_manager.resolve_variable(&mut core, "G_O", addr).expect("Failed to resolve G_O");
    
    assert!(info.name == "G_O");
    assert!(info.kind == "Struct" || info.kind == "Enum", "Expected Struct (or Enum) for Option, got {}", info.kind);
    
    let members = info.members.as_ref().expect("Option members missing");
    // Option<u32> should have variants like None and Some
    assert!(members.iter().any(|m| m.name == "None"), "None variant missing");
    assert!(members.iter().any(|m| m.name == "Some"), "Some variant missing");
}
