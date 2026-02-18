use aether_core::TaskState;
use std::path::Path;

/// Formats a line of memory for the hex view.
/// Returns (address_str, hex_str, ascii_str)
pub fn format_memory_line(address: u64, chunk: &[u8]) -> (String, String, String) {
    let addr_str = format!("{:08X}", address);

    let hex_part: String = chunk.iter()
        .map(|b| format!("{:02X} ", b))
        .collect();

    let ascii_part: String = chunk.iter()
        .map(|b| if *b >= 32 && *b <= 126 { *b as char } else { '.' })
        .collect();

    (addr_str, format!("{:48}", hex_part), ascii_part)
}

/// Returns a user-friendly string for the task state.
pub fn get_task_state_display(state: TaskState) -> &'static str {
    match state {
        TaskState::Running => "▶ Running",
        TaskState::Ready => "🟢 Ready",
        TaskState::Blocked => "🟡 Blocked",
        TaskState::Suspended => "⚪ Suspended",
        TaskState::Deleted => "🔴 Deleted",
        TaskState::Pending => "⏳ Pending",
        TaskState::Unknown => "❓ Unknown",
    }
}

/// Returns a shortened filename from a full path for display.
pub fn get_display_location(file: Option<&str>, line: Option<u64>) -> String {
    if let (Some(file), Some(line)) = (file, line) {
        let path = Path::new(file);
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        format!("{}:{}", filename, line)
    } else {
        "??".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_memory_line() {
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let (addr, hex, ascii) = format_memory_line(0x1000, &data);
        assert_eq!(addr, "00001000");
        assert!(hex.starts_with("DE AD BE EF "));
        assert_eq!(ascii, "....");
    }

    #[test]
    fn test_task_state_display() {
        assert_eq!(get_task_state_display(TaskState::Running), "▶ Running");
        assert_eq!(get_task_state_display(TaskState::Blocked), "🟡 Blocked");
    }

    #[test]
    fn test_display_location() {
        assert_eq!(get_display_location(Some("/path/to/main.rs"), Some(42)), "main.rs:42");
        assert_eq!(get_display_location(None, None), "??");
    }
}
