pub mod freertos;

use crate::{TaskInfo};
use crate::symbols::SymbolManager;
use anyhow::Result;
use probe_rs::MemoryInterface;

pub trait RtosAware: Send {
    fn name(&self) -> &str;
    fn get_tasks(&self, core: &mut dyn MemoryInterface, symbols: &SymbolManager) -> Result<Vec<TaskInfo>>;
}

pub fn detect_rtos(symbols: &SymbolManager) -> Option<Box<dyn RtosAware>> {
    // Check if FreeRTOS symbols are present
    if symbols.lookup_symbol("pxReadyTasksLists").is_some() {
        return Some(Box::new(freertos::FreeRtos::new()));
    }
    None
}
