use super::RtosAware;
use crate::symbols::SymbolManager;
use crate::{TaskInfo, TaskState};
use anyhow::Result;
use probe_rs::MemoryInterface;

pub struct Embassy;

impl Default for Embassy {
    fn default() -> Self {
        Self::new()
    }
}

impl Embassy {
    pub fn new() -> Self {
        Self
    }
}

impl RtosAware for Embassy {
    fn name(&self) -> &str {
        "Embassy"
    }

    fn get_tasks(
        &self,
        _core: &mut dyn MemoryInterface,
        symbols: &SymbolManager,
    ) -> Result<Vec<TaskInfo>> {
        let mut tasks = Vec::new();

        // Embassy doesn't have a single "Task" structure like FreeRTOS.
        // It uses a pool of tasks (Executors).
        // Common symbol for global executor: __embassy_executor_global

        if let Some(executor_ptr) = symbols.lookup_symbol("__embassy_executor_global") {
            // This is a rough estimation of how we'd find the tasks.
            // Usually involve iterating through the executor's task list.
            // Placeholder logic for now to show the pattern.

            // 1. Read task list head from executor
            // 2. Iterate and resolve future types using DWARF

            tasks.push(TaskInfo {
                name: "Embassy Executor".to_string(),
                priority: 0,
                state: TaskState::Running,
                stack_usage: 0,
                stack_size: 0,
                handle: executor_ptr as u32,
                task_type: crate::TaskType::Async,
            });
        }

        Ok(tasks)
    }
}
