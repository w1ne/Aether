use aether_core::{SessionHandle, DebugCommand, DebugEvent, TaskState};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_scenario_halt_and_inspect() {
    // 1. Initialize Session in Test Mode
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    
    // 2. Simulate User Clicking "Halt"
    handle.send(DebugCommand::Halt).expect("Failed to send Halt");
    
    // 3. Verify Logic: Core should receive Halt command
    let cmd = cmd_rx.try_recv().expect("Core did not receive Halt command");
    assert!(matches!(cmd, DebugCommand::Halt));
    
    // 4. Verify UI/Client starts listening
    let mut receiver = handle.subscribe();
    
    // 5. Simulate Target Halting
    let pc_val = 0x08001234;
    event_tx.send(DebugEvent::Halted { pc: pc_val }).expect("Failed to broadcast Halted event");
    
    // 6. Verify UI/Client receives the Halted event
    let event: DebugEvent = timeout(Duration::from_millis(100), receiver.recv())
        .await
        .expect("Timeout waiting for Halted event")
        .expect("Failed to receive event");
        
    match event {
        DebugEvent::Halted { pc } => assert_eq!(pc, pc_val),
        _ => panic!("Expected Halted event, got {:?}", event),
    }
}

#[tokio::test]
async fn test_scenario_rtos_tasks_discovery() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    
    // 1. User switches to Tasks Tab (triggers GetTasks)
    handle.send(DebugCommand::GetTasks).expect("Failed to send GetTasks");
    
    // 2. Verify Command
    let cmd = cmd_rx.try_recv().expect("Core did not receive GetTasks command");
    assert!(matches!(cmd, DebugCommand::GetTasks));
    
    // 3. Verify propagation starts
    let mut receiver = handle.subscribe();
    
    // 4. Simulate RTOS Manager finding tasks
    let mock_tasks = vec![
        aether_core::TaskInfo {
            name: "MainTask".to_string(),
            priority: 4,
            state: TaskState::Running,
            stack_usage: 128,
            stack_size: 1024,
            handle: 0x20001000,
        },
        aether_core::TaskInfo {
            name: "IdleTask".to_string(),
            priority: 0,
            state: TaskState::Ready,
            stack_usage: 64,
            stack_size: 512,
            handle: 0x20002000,
        },
    ];
    
    event_tx.send(DebugEvent::Tasks(mock_tasks.clone())).expect("Failed to broadcast Tasks event");
    
    // 5. Verify propagation
    let event: DebugEvent = timeout(Duration::from_millis(100), receiver.recv())
        .await
        .expect("Timeout waiting for Tasks event")
        .expect("Failed to receive event");
        
    match event {
        DebugEvent::Tasks(tasks) => {
            assert_eq!(tasks.len(), 2);
            assert_eq!(tasks[0].name, "MainTask");
            assert_eq!(tasks[1].name, "IdleTask");
        },
        _ => panic!("Expected Tasks event, got {:?}", event),
    }
}

#[tokio::test]
async fn test_scenario_agent_api_automation() {
    // This test verifies that an external API call correctly drives the core
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    
    // Simulate Agent API server starting and receiving a Resume request
    // (We don't need the full gRPC stack here, just verify the logic)
    let api_client_handle = handle.clone();
    
    // API Call: Resume
    api_client_handle.send(DebugCommand::Resume).expect("API failed to send Resume");
    
    // Verify Core received it
    let cmd = cmd_rx.try_recv().expect("Core did not receive Resume from API");
    assert!(matches!(cmd, DebugCommand::Resume));
    
    // Verify both UI and API starts listening
    let mut ui_receiver = handle.subscribe();
    
    // Core responds with Resumed event
    event_tx.send(DebugEvent::Resumed).expect("Failed to broadcast Resumed event");
    
    // Verify both UI and API (if they subscribe) receive it
    let event: DebugEvent = timeout(Duration::from_millis(100), ui_receiver.recv())
        .await
        .expect("Timeout waiting for Resumed event")
        .expect("Failed to receive event");
        
    assert!(matches!(event, DebugEvent::Resumed));
}

#[tokio::test]
async fn test_scenario_stack_unwind() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    
    // 1. Send GetStack command
    handle.send(DebugCommand::GetStack).expect("Failed to send GetStack");
    
    // 2. Verify Core received it
    let cmd = cmd_rx.try_recv().expect("Core did not receive GetStack");
    assert!(matches!(cmd, DebugCommand::GetStack));
    
    // 3. Verify listeners are ready
    let mut receiver = handle.subscribe();
    
    // 4. Simulate Core sending Stack event with multiple frames
    let mock_stack = vec![
        aether_core::StackFrame {
            id: 0,
            function_name: "main".to_string(),
            source_file: Some("src/main.rs".to_string()),
            line: Some(42),
            pc: 0x08001000,
            sp: 0x20004000,
        },
        aether_core::StackFrame {
            id: 1,
            function_name: "Reset_Handler".to_string(),
            source_file: Some("src/startup.rs".to_string()),
            line: Some(10),
            pc: 0x08000100,
            sp: 0x20004020,
        }
    ];
    event_tx.send(DebugEvent::Stack(mock_stack)).expect("Failed to send Stack event");
    
    // 5. Verify UI received Stack event
    let event: DebugEvent = timeout(Duration::from_millis(100), receiver.recv())
        .await
        .expect("Timeout waiting for Stack event")
        .expect("Failed to receive event");
        
    match event {
        DebugEvent::Stack(frames) => {
            assert_eq!(frames.len(), 2);
            assert_eq!(frames[0].function_name, "main");
            assert_eq!(frames[0].line, Some(42));
            assert_eq!(frames[1].function_name, "Reset_Handler");
        },
        _ => panic!("Expected Stack event, got {:?}", event),
    }
}

#[tokio::test]
async fn test_scenario_trace_streaming() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    
    // 1. Enable Trace
    let config = aether_core::trace::TraceConfig {
        core_frequency: 16_000_000,
        trace_frequency: 2_000_000,
        itm_ports: vec![0],
    };
    handle.send(DebugCommand::EnableTrace(config.clone())).expect("Failed to send EnableTrace");
    
    // 2. Verify Core received command
    let cmd = cmd_rx.try_recv().expect("Core did not receive EnableTrace");
    if let DebugCommand::EnableTrace(c) = cmd {
        assert_eq!(c.core_frequency, config.core_frequency);
    } else {
        panic!("Expected EnableTrace command");
    }
    
    // 3. Subscribe to events
    let mut receiver = handle.subscribe();
    
    // 4. Simulate periodic trace data arriving from core
    let trace_bytes = vec![0x1, 0x2, 0x3, 0x4];
    event_tx.send(DebugEvent::TraceData(trace_bytes.clone())).expect("Failed to send TraceData");
    
    // 5. Verify UI/Client receives trace data
    let event: DebugEvent = timeout(Duration::from_millis(100), receiver.recv())
        .await
        .expect("Timeout waiting for TraceData event")
        .expect("Failed to receive event");
        
    match event {
        DebugEvent::TraceData(data) => {
            assert_eq!(data, trace_bytes);
        },
        _ => panic!("Expected TraceData event, got {:?}", event),
    }
}

#[tokio::test]
async fn test_scenario_flash_full() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Trigger Flash
    let path = std::path::PathBuf::from("fake_firmware.elf");
    handle.send(DebugCommand::StartFlashing(path.clone())).expect("Failed to send StartFlashing");
    
    // 2. Verify Core received command
    let cmd = cmd_rx.try_recv().expect("Core did not receive StartFlashing");
    if let DebugCommand::StartFlashing(p) = cmd {
        assert_eq!(p, path);
    } else {
        panic!("Expected StartFlashing command");
    }
    
    // 3. Simulate Flash Progress
    event_tx.send(DebugEvent::FlashStatus("Erasing...".to_string())).unwrap();
    event_tx.send(DebugEvent::FlashProgress(0.5)).unwrap();
    event_tx.send(DebugEvent::FlashDone).unwrap();
    
    // 4. Verify propagation
    let ev1 = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    assert!(matches!(ev1, DebugEvent::FlashStatus(_)));
    
    let ev2 = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    assert!(matches!(ev2, DebugEvent::FlashProgress(_)));
    
    let ev3 = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    assert!(matches!(ev3, DebugEvent::FlashDone));
}

#[tokio::test]
async fn test_scenario_memory_stress() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Large Write
    let large_data = vec![0xAA; 1024];
    handle.send(DebugCommand::WriteMemory(0x20000000, large_data.clone())).unwrap();
    
    let cmd = cmd_rx.try_recv().expect("Core did not receive WriteMemory");
    if let DebugCommand::WriteMemory(addr, data) = cmd {
        assert_eq!(addr, 0x20000000);
        assert_eq!(data.len(), 1024);
    } else {
        panic!("Expected WriteMemory command");
    }
    
    // 2. Large Read Request
    handle.send(DebugCommand::ReadMemory(0x20000000, 1024)).unwrap();
    
    let cmd = cmd_rx.try_recv().expect("Core did not receive ReadMemory");
    if let DebugCommand::ReadMemory(addr, size) = cmd {
        assert_eq!(addr, 0x20000000);
        assert_eq!(size, 1024);
    } else {
        panic!("Expected ReadMemory command");
    }
    
    // 3. Simulate Data Arrival
    event_tx.send(DebugEvent::MemoryData(0x20000000, large_data.clone())).unwrap();
    
    // 4. Verify propagation
    let event = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    match event {
        DebugEvent::MemoryData(addr, data) => {
            assert_eq!(addr, 0x20000000);
            assert_eq!(data.len(), 1024);
            assert_eq!(data, large_data);
        },
        _ => panic!("Expected MemoryData event"),
    }
}
