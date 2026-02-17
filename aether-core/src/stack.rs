use probe_rs::{Core, MemoryInterface};
use probe_rs_debug::StackFrame as ProbeStackFrame;
use serde::{Serialize, Deserialize};
use gimli::{
    BaseAddresses, UnwindSection, UnwindContext, RunTimeEndian, 
    DebugFrame
};
use object::{Object, ObjectSection};
use std::borrow::Cow;
use crate::symbols::SymbolManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: u64,
    pub function_name: String,
    pub source_file: Option<String>,
    pub line: Option<u64>,
    pub pc: u32,
    pub sp: u32,
}

impl From<&ProbeStackFrame> for StackFrame {
    fn from(frame: &ProbeStackFrame) -> Self {
        StackFrame {
            id: 0, 
            function_name: frame.function_name.clone(),
            source_file: frame.source_location.as_ref().map(|l| l.path.to_string_lossy().to_string()),
            line: frame.source_location.as_ref().and_then(|l| l.line),
            pc: frame.pc.to_string().parse::<u64>().unwrap_or(0) as u32,
            sp: 0, 
        }
    }
}

pub fn unwind_stack(core: &mut Core, symbol_manager: &SymbolManager) -> Result<Vec<StackFrame>, String> {
    // 1. Initial State
    let mut frames = Vec::new();
    
    // Get current registers
    let pc_val: u64 = core.read_core_reg(core.program_counter()).map_err(|e| e.to_string())?;
    let sp_val: u64 = core.read_core_reg(core.stack_pointer()).map_err(|e| e.to_string())?;
    // We might need LR for leaf functions or if DWARF is missing
    let lr_val: u64 = core.read_core_reg(core.return_address()).map_err(|e| e.to_string())?;
    
    // Current frame (Top of Stack)
    // Try to resolve function name for PC
    let func_name = if let Some(info) = symbol_manager.lookup(pc_val) {
         info.function.unwrap_or_else(|| format!("0x{:08x}", pc_val))
    } else {
         format!("0x{:08x}", pc_val)
    };

    let source_loc = symbol_manager.lookup(pc_val);

    frames.push(StackFrame {
        id: 0,
        function_name: func_name,
        source_file: source_loc.as_ref().map(|l| l.file.to_string_lossy().to_string()),
        line: source_loc.as_ref().map(|l| l.line as u64),
        pc: pc_val as u32,
        sp: sp_val as u32,
    });

    // 2. Load ELF and DWARF for Unwinding
    let elf_data = if let Some(data) = symbol_manager.elf_data() {
        data
    } else {
        // If no symbols, we stop here (Basic 1-frame stack)
        return Ok(frames);
    };

    let obj = object::File::parse(elf_data).map_err(|e| e.to_string())?;
    let endian = if obj.is_little_endian() { RunTimeEndian::Little } else { RunTimeEndian::Big };

    // Try .debug_frame, then .eh_frame
    let debug_frame_section = obj.section_by_name(".debug_frame").map(|s| s.uncompressed_data().unwrap_or(Cow::Borrowed(&[])));
    let _eh_frame_section = obj.section_by_name(".eh_frame").map(|s| s.uncompressed_data().unwrap_or(Cow::Borrowed(&[])));

    // Create UnwindContext
    let mut ctx = UnwindContext::new();
    
    // Register state state (DWARF register numbers)
    // Cortex-M: 13=SP, 14=LR, 15=PC
    let mut current_pc = pc_val;
    let mut current_sp = sp_val;
    let current_lr = lr_val;
    
    // Limit depth
    for _ in 0..20 {
        let mut unwound = false;

        if let Some(section_data) = &debug_frame_section {
             let debug_frame = DebugFrame::new(section_data, endian);
             let mut bases = BaseAddresses::default();
             bases = bases.set_text(0); // Assuming 0 for now
             if let Ok(fde) = debug_frame.fde_for_address(&bases, current_pc, |f, b, o| f.cie_from_offset(b, o)) {
                      if let Ok(row) = fde.unwind_info_for_address(&debug_frame, &bases, &mut ctx, current_pc) {
                           // Evaluate CFA (Canonical Frame Address) - usually SP of caller
                           let cfa = match row.cfa() {
                               gimli::CfaRule::RegisterAndOffset { register, offset } => {
                                   let reg_val = if register.0 == 13 { current_sp } else { 0 }; // TODO: Handle other regs
                                   (reg_val as i64 + offset) as u64
                               }
                               _ => current_sp // Fallback
                           };
                           
                           // Evaluate Return Address (RA) -> PC of caller
                           // Usually stores in LR (14) or on stack
                           let ra_rule = row.register(gimli::Register(14)); // LR
                           let caller_pc = match ra_rule {
                               gimli::RegisterRule::Undefined => {
                                   // If Undefined, maybe we are at bottom or uses LR directly
                                   if current_lr != 0 { current_lr } else { 0 }
                               },
                               gimli::RegisterRule::SameValue => current_lr,
                               gimli::RegisterRule::Offset(offset) => {
                                   // Saved at CFA + offset
                                   let addr = (cfa as i64 + offset) as u64;
                                   match core.read_word_32(addr) {
                                       Ok(val) => val as u64,
                                       Err(_) => 0
                                   }
                               },
                               gimli::RegisterRule::ValOffset(offset) => (cfa as i64 + offset) as u64,
                               gimli::RegisterRule::Register(reg) => {
                                    if reg.0 == 14 { current_lr } else { 0 } // Simplified
                               },
                               _ => 0
                           };

                           if caller_pc == 0 || caller_pc == current_pc {
                               break; // Stop unwinding
                           }
                           
                           // Update state for next frame
                           current_pc = caller_pc;
                           current_sp = cfa;
                           // current_lr should be updated too if possible, but simpler is ok for now
                           unwound = true;
                      }
                 }
            }
        
        // If DWARF failed, maybe try primitive LR unwinding for one step?
        if !unwound {
             // Basic leaf function handling: if we are in a leaf, LR holds the caller
             // But we simulate this inside loop usually. 
             // If we haven't found DWARF, we assume we can't unwind further safely.
             break;
        }
        
        // Resolve symbol for new PC
        let func_name = if let Some(info) = symbol_manager.lookup(current_pc) {
             info.function.unwrap_or_else(|| format!("0x{:08x}", current_pc))
        } else {
             format!("0x{:08x}", current_pc)
        };
        let source_loc = symbol_manager.lookup(current_pc);
        
        frames.push(StackFrame {
            id: frames.len() as u64,
            function_name: func_name,
            source_file: source_loc.as_ref().map(|l| l.file.to_string_lossy().to_string()),
            line: source_loc.as_ref().map(|l| l.line as u64),
            pc: current_pc as u32,
            sp: current_sp as u32,
        });
        
        // Stop if we hit typical end-of-stack markers (e.g. 0xFFFFFFFF or 0)
        if current_pc == 0 || current_pc == 0xFFFFFFFF {
            break;
        }
    }
    
    Ok(frames)
}
