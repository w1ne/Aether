pub mod freertos;
pub mod embassy;

use crate::{TaskInfo};
use crate::symbols::SymbolManager;
use anyhow::Result;
use probe_rs::MemoryInterface;

pub trait RtosAware: Send {
    fn name(&self) -> &str;
    fn get_tasks(&self, core: &mut dyn MemoryInterface, symbols: &SymbolManager) -> Result<Vec<TaskInfo>>;
}

pub fn detect_rtos(symbols: &SymbolManager) -> Option<Box<dyn RtosAware>> {
    // 1. FreeRTOS
    if symbols.lookup_symbol("pxReadyTasksLists").is_some() {
        return Some(Box::new(freertos::FreeRtos::new()));
    }

    // 2. Embassy
    if symbols.lookup_symbol("__embassy_executor_global").is_some() {
        return Some(Box::new(embassy::Embassy::new()));
    }

    None
}
