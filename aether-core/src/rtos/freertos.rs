use super::RtosAware;
use crate::symbols::SymbolManager;
use crate::{TaskInfo, TaskState};
use anyhow::Result;
use probe_rs::MemoryInterface;

pub struct FreeRtos;

impl Default for FreeRtos {
    fn default() -> Self {
        Self::new()
    }
}

impl FreeRtos {
    pub fn new() -> Self {
        Self
    }

    fn read_list(
        &self,
        core: &mut dyn MemoryInterface,
        list_addr: u64,
        state: TaskState,
        tasks: &mut Vec<TaskInfo>,
    ) -> Result<()> {
        // FreeRTOS List_t structure (simplified):
        // uxNumberOfItems (u32)
        // pxIndex (pointer)
        // xListEnd (MiniListItem_t)

        let num_items: u32 = core.read_word_32(list_addr)?;
        if num_items == 0 {
            return Ok(());
        }

        // xListEnd starts at offset 8 (after 4-byte count and 4-byte pointer)
        // MiniListItem_t: xItemValue (u32), pxNext (pointer), pxPrevious (pointer)
        let list_end_addr = list_addr + 8;
        let mut current_item_addr: u32 = core.read_word_32(list_end_addr + 4)?; // pxNext

        for _ in 0..num_items {
            if current_item_addr == 0 || current_item_addr == list_end_addr as u32 {
                break;
            }

            // ListItem_t: xItemValue (u32), pxNext (pointer), pxPrevious (pointer), pvOwner (pointer), pvContainer (pointer)
            // pvOwner is at offset 12
            let tcb_addr: u32 = core.read_word_32(current_item_addr as u64 + 12)?;

            if tcb_addr != 0 {
                if let Ok(task) = self.read_tcb(core, tcb_addr as u64, state) {
                    tasks.push(task);
                }
            }

            // Move to next item
            current_item_addr = core.read_word_32(current_item_addr as u64 + 4)?;
        }

        Ok(())
    }

    fn read_tcb(
        &self,
        core: &mut dyn MemoryInterface,
        tcb_addr: u64,
        state: TaskState,
    ) -> Result<TaskInfo> {
        // TCB_t structure (simplified, may vary by FreeRTOS version/config):
        // pxTopOfStack (offset 0)
        // ...
        // uxPriority (offset 44)
        // pxStack (offset 48)
        // pcTaskName (offset 52, size configMAX_TASK_NAME_LEN)

        let top_of_stack: u32 = core.read_word_32(tcb_addr)?;
        let priority: u32 = core.read_word_32(tcb_addr + 44)?;
        let stack_start: u32 = core.read_word_32(tcb_addr + 48)?;

        let mut name_bytes = [0u8; 16];
        core.read_8(tcb_addr + 52, &mut name_bytes)?;
        let name = String::from_utf8_lossy(&name_bytes).trim_matches(char::from(0)).to_string();

        // Calculate stack usage (simple version: current at time of switch)
        // stack_start is the bottom (lowest address if stack grows down, but pxStack is usually the beginning of the allocated block)
        // In ARM Cortex-M, stack grows down. pxStack points to the lowest address.
        // So stack_size = (end_of_stack_allocated - pxStack)
        // But we don't always know end_of_stack easily without config.
        // However, (top_of_stack - stack_start) gives us a window.

        // High water mark scan
        let _stack_size = 0; // Unknown without more TCB parsing or config
        let high_water_mark =
            self.scan_high_water_mark(core, stack_start as u64, top_of_stack as u64).unwrap_or(0);

        let stack_usage = top_of_stack.saturating_sub(stack_start);
        Ok(TaskInfo {
            name,
            priority,
            state,
            stack_usage,
            stack_size: high_water_mark as u32, // repurposed to show the "Peak" for now
            handle: tcb_addr as u32,
            task_type: crate::TaskType::Thread,
        })
    }

    fn scan_high_water_mark(
        &self,
        core: &mut dyn MemoryInterface,
        stack_start: u64,
        top_of_stack: u64,
    ) -> Result<u64> {
        // Scan from stack_start upwards matching 0xa5 pattern
        const CHUNK_SIZE: usize = 256;
        let mut buffer = [0u8; CHUNK_SIZE];
        let mut current_addr = stack_start;
        let mut total_unused = 0;

        while current_addr < top_of_stack {
            let to_read = std::cmp::min(CHUNK_SIZE, (top_of_stack - current_addr) as usize);
            if core.read_8(current_addr, &mut buffer[0..to_read]).is_err() {
                break;
            }

            for &byte in &buffer[0..to_read] {
                if byte == 0xa5 {
                    total_unused += 1;
                } else {
                    return Ok(total_unused);
                }
            }
            current_addr += to_read as u64;
        }

        Ok(total_unused)
    }
}

impl RtosAware for FreeRtos {
    fn name(&self) -> &str {
        "FreeRTOS"
    }

    fn get_tasks(
        &self,
        core: &mut dyn MemoryInterface,
        symbols: &SymbolManager,
    ) -> Result<Vec<TaskInfo>> {
        let mut tasks = Vec::new();

        // 1. pxReadyTasksLists
        if let Some(ready_lists_addr) = symbols.lookup_symbol("pxReadyTasksLists") {
            // It's an array of configMAX_PRIORITIES lists
            // Let's assume 32 priorities for now, or check for symbol if available
            for i in 0..32 {
                let list_addr = ready_lists_addr + (i * 20); // List_t is 20 bytes on 32-bit
                let _ = self.read_list(core, list_addr, TaskState::Ready, &mut tasks);
            }
        }

        // 2. xDelayedTaskList1
        if let Some(addr) = symbols.lookup_symbol("xDelayedTaskList1") {
            let _ = self.read_list(core, addr, TaskState::Blocked, &mut tasks);
        }

        // 3. xDelayedTaskList2
        if let Some(addr) = symbols.lookup_symbol("xDelayedTaskList2") {
            let _ = self.read_list(core, addr, TaskState::Blocked, &mut tasks);
        }

        // 4. xSuspendedTaskList
        if let Some(addr) = symbols.lookup_symbol("xSuspendedTaskList") {
            let _ = self.read_list(core, addr, TaskState::Suspended, &mut tasks);
        }

        // 5. Identify the running task
        if let Some(current_tcb_addr_ptr) = symbols.lookup_symbol("pxCurrentTCB") {
            let current_tcb_addr: u32 = core.read_word_32(current_tcb_addr_ptr)?;
            for task in tasks.iter_mut() {
                if task.handle == current_tcb_addr {
                    task.state = TaskState::Running;
                }
            }
        }

        Ok(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockMemory {
        pub data: std::collections::HashMap<u64, u8>,
    }

    impl MockMemory {
        pub fn new() -> Self {
            Self { data: std::collections::HashMap::new() }
        }

        fn set_word_32(&mut self, addr: u64, val: u32) {
            for (i, byte) in val.to_le_bytes().iter().enumerate() {
                self.data.insert(addr + i as u64, *byte);
            }
        }

        fn set_bytes(&mut self, addr: u64, bytes: &[u8]) {
            for (i, &byte) in bytes.iter().enumerate() {
                self.data.insert(addr + i as u64, byte);
            }
        }
    }

    impl MemoryInterface for MockMemory {
        fn read_word_8(&mut self, address: u64) -> Result<u8, probe_rs::Error> {
            let mut b = [0u8; 1];
            self.read_8(address, &mut b)?;
            Ok(b[0])
        }
        fn read_word_16(&mut self, address: u64) -> Result<u16, probe_rs::Error> {
            let mut b = [0u8; 2];
            self.read_8(address, &mut b)?;
            Ok(u16::from_le_bytes(b))
        }
        fn read_word_32(&mut self, address: u64) -> Result<u32, probe_rs::Error> {
            let mut b = [0u8; 4];
            self.read_8(address, &mut b)?;
            Ok(u32::from_le_bytes(b))
        }
        fn read_word_64(&mut self, address: u64) -> Result<u64, probe_rs::Error> {
            let mut b = [0u8; 8];
            self.read_8(address, &mut b)?;
            Ok(u64::from_le_bytes(b))
        }
        fn write_word_8(&mut self, address: u64, data: u8) -> Result<(), probe_rs::Error> {
            self.write_8(address, &[data])
        }
        fn write_word_16(&mut self, address: u64, data: u16) -> Result<(), probe_rs::Error> {
            self.write_8(address, &data.to_le_bytes())
        }
        fn write_word_32(&mut self, address: u64, data: u32) -> Result<(), probe_rs::Error> {
            self.write_8(address, &data.to_le_bytes())
        }
        fn write_word_64(&mut self, address: u64, data: u64) -> Result<(), probe_rs::Error> {
            self.write_8(address, &data.to_le_bytes())
        }
        fn read_8(&mut self, address: u64, data: &mut [u8]) -> Result<(), probe_rs::Error> {
            for (i, byte) in data.iter_mut().enumerate() {
                *byte = *self.data.get(&(address + i as u64)).unwrap_or(&0);
            }
            Ok(())
        }
        fn write_8(&mut self, address: u64, data: &[u8]) -> Result<(), probe_rs::Error> {
            for (i, &byte) in data.iter().enumerate() {
                self.data.insert(address + i as u64, byte);
            }
            Ok(())
        }
        fn read_16(&mut self, address: u64, data: &mut [u16]) -> Result<(), probe_rs::Error> {
            for (i, word) in data.iter_mut().enumerate() {
                *word = self.read_word_16(address + (i * 2) as u64)?;
            }
            Ok(())
        }
        fn write_16(&mut self, address: u64, data: &[u16]) -> Result<(), probe_rs::Error> {
            for (i, &word) in data.iter().enumerate() {
                self.write_word_16(address + (i * 2) as u64, word)?;
            }
            Ok(())
        }
        fn read_32(&mut self, address: u64, data: &mut [u32]) -> Result<(), probe_rs::Error> {
            for (i, word) in data.iter_mut().enumerate() {
                *word = self.read_word_32(address + (i * 4) as u64)?;
            }
            Ok(())
        }
        fn write_32(&mut self, address: u64, data: &[u32]) -> Result<(), probe_rs::Error> {
            for (i, &word) in data.iter().enumerate() {
                self.write_word_32(address + (i * 4) as u64, word)?;
            }
            Ok(())
        }
        fn read_64(&mut self, address: u64, data: &mut [u64]) -> Result<(), probe_rs::Error> {
            for (i, word) in data.iter_mut().enumerate() {
                *word = self.read_word_64(address + (i * 8) as u64)?;
            }
            Ok(())
        }
        fn write_64(&mut self, address: u64, data: &[u64]) -> Result<(), probe_rs::Error> {
            for (i, &word) in data.iter().enumerate() {
                self.write_word_64(address + (i * 8) as u64, word)?;
            }
            Ok(())
        }
        fn flush(&mut self) -> Result<(), probe_rs::Error> {
            Ok(())
        }
        fn supports_native_64bit_access(&mut self) -> bool {
            false
        }
        fn supports_8bit_transfers(&self) -> Result<bool, probe_rs::Error> {
            Ok(true)
        }
    }

    #[test]
    fn test_freertos_scanning() {
        let mut mock = MockMemory::new();
        let _syms = SymbolManager::new(); // Empty for now, we'll manually use addrs

        // Mock a ReadyTask List (at 0x2000)
        // uxNumberOfItems = 1
        mock.set_word_32(0x2000, 1);
        // pxIndex = 0x2008 (points to xListEnd)
        mock.set_word_32(0x2004, 0x2008);
        // xListEnd (at 0x2008): xItemValue = 0xFFFFFFFF, pxNext = 0x3000 (first item), pxPrevious = 0x3000
        mock.set_word_32(0x2008, 0xFFFFFFFF);
        mock.set_word_32(0x200C, 0x3000);
        mock.set_word_32(0x2010, 0x3000);

        // Mock a ListItem (at 0x3000)
        // xItemValue = 1, pxNext = 0x2008, pxPrevious = 0x2008, pvOwner = 0x4000 (TCB), pvContainer = 0x2000
        mock.set_word_32(0x3000, 1);
        mock.set_word_32(0x3004, 0x2008);
        mock.set_word_32(0x3008, 0x2008);
        mock.set_word_32(0x300C, 0x4000); // pvOwner points to TCB

        // Mock a TCB (at 0x4000)
        // pxTopOfStack = 0x3010 (offset 0)
        mock.set_word_32(0x4000, 0x3010);
        // priority (offset 44) = 5
        mock.set_word_32(0x4000 + 44, 5);
        // pxStack (offset 48) = 0x3000
        mock.set_word_32(0x4000 + 48, 0x3000);
        // name (offset 52) = "TestTask"
        let mut name = [0u8; 16];
        name[0..8].copy_from_slice(b"TestTask");
        mock.set_bytes(0x4000 + 52, &name);

        // Fill stack with some 0xa5 pattern
        // 0x3000-0x3004: 0xa5 (unused)
        // 0x3004-0x3010: something else (used)
        mock.set_bytes(0x3000, &[0xa5, 0xa5, 0xa5, 0xa5, 0x11, 0x22, 0x33, 0x44]);

        let freertos = FreeRtos::new();
        let mut tasks = Vec::new();
        freertos.read_list(&mut mock, 0x2000, TaskState::Ready, &mut tasks).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "TestTask");
        assert_eq!(tasks[0].priority, 5);
        assert_eq!(tasks[0].state, TaskState::Ready);
        assert_eq!(tasks[0].handle, 0x4000);
        assert_eq!(tasks[0].stack_usage, 0x10); // 0x3010 - 0x3000
        assert_eq!(tasks[0].stack_size, 4); // High water mark: 4 unused bytes
    }
}
