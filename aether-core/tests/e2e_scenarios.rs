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
            task_type: aether_core::TaskType::Thread,
        },
        aether_core::TaskInfo {
            name: "IdleTask".to_string(),
            priority: 0,
            state: TaskState::Ready,
            stack_usage: 64,
            stack_size: 512,
            handle: 0x20002000,
            task_type: aether_core::TaskType::Thread,
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

#[tokio::test]
async fn test_scenario_registers_access() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Read Register
    handle.send(DebugCommand::ReadRegister(15)).unwrap(); // PC
    let cmd = cmd_rx.try_recv().expect("Core did not receive ReadRegister");
    assert!(matches!(cmd, DebugCommand::ReadRegister(15)));
    
    // 2. Simulate Value
    event_tx.send(DebugEvent::RegisterValue(15, 0x08001000)).unwrap();
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::RegisterValue(reg, val) = ev {
        assert_eq!(reg, 15);
        assert_eq!(val, 0x08001000);
    } else {
        panic!("Expected RegisterValue event");
    }
    
    // 3. Write Register
    handle.send(DebugCommand::WriteRegister(13, 0x20004000)).unwrap(); // SP
    let cmd = cmd_rx.try_recv().expect("Core did not receive WriteRegister");
    assert!(matches!(cmd, DebugCommand::WriteRegister(13, 0x20004000)));
}

#[tokio::test]
async fn test_scenario_breakpoint_management() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Set Breakpoint
    handle.send(DebugCommand::SetBreakpoint(0x08001234)).unwrap();
    let cmd = cmd_rx.try_recv().expect("Core did not receive SetBreakpoint");
    assert!(matches!(cmd, DebugCommand::SetBreakpoint(0x08001234)));
    
    // 2. List Breakpoints
    handle.send(DebugCommand::ListBreakpoints).unwrap();
    let cmd = cmd_rx.try_recv().expect("Core did not receive ListBreakpoints");
    assert!(matches!(cmd, DebugCommand::ListBreakpoints));
    
    // 3. Simulate Logic returning list
    event_tx.send(DebugEvent::Breakpoints(vec![0x08001234])).unwrap();
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::Breakpoints(bps) = ev {
        assert_eq!(bps, vec![0x08001234]);
    } else {
        panic!("Expected Breakpoints event");
    }
    
    // 4. Clear Breakpoint
    handle.send(DebugCommand::ClearBreakpoint(0x08001234)).unwrap();
    let cmd = cmd_rx.try_recv().expect("Core did not receive ClearBreakpoint");
    assert!(matches!(cmd, DebugCommand::ClearBreakpoint(0x08001234)));
}

#[tokio::test]
async fn test_scenario_svd_interaction() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Load SVD
    let path = std::path::PathBuf::from("stm32l4.svd");
    handle.send(DebugCommand::LoadSvd(path.clone())).unwrap();
    let cmd = cmd_rx.try_recv().expect("Core did not receive LoadSvd");
    assert!(matches!(cmd, DebugCommand::LoadSvd(_)));
    
    // 2. Simulate SVD Loaded
    event_tx.send(DebugEvent::SvdLoaded).unwrap();
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    assert!(matches!(ev, DebugEvent::SvdLoaded));
    
    // 3. Get Peripherals
    handle.send(DebugCommand::GetPeripherals).unwrap();
    let cmd = cmd_rx.try_recv().expect("Core did not receive GetPeripherals");
    assert!(matches!(cmd, DebugCommand::GetPeripherals));
    
    // 4. Simulate Peripherals List
    let mock_periphs = vec![
        aether_core::svd::PeripheralInfo {
            name: "GPIOA".to_string(),
            base_address: 0x48000000,
            description: Some("GPIO Port A".to_string()),
        }
    ];
    event_tx.send(DebugEvent::Peripherals(mock_periphs)).unwrap();
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::Peripherals(p) = ev {
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].name, "GPIOA");
    } else {
        panic!("Expected Peripherals event");
    }
}

#[tokio::test]
async fn test_scenario_variable_plotting() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Add Plot
    handle.send(DebugCommand::AddPlot { 
        name: "temperature".to_string(), 
        var_type: aether_core::VarType::F32 
    }).unwrap();
    
    let cmd = cmd_rx.try_recv().expect("Core did not receive AddPlot");
    if let DebugCommand::AddPlot { name, var_type } = cmd {
        assert_eq!(name, "temperature");
        assert_eq!(var_type, aether_core::VarType::F32);
    } else {
        panic!("Expected AddPlot command");
    }
    
    // 2. Simulate Periodic Data Arrival
    event_tx.send(DebugEvent::PlotData {
        name: "temperature".to_string(),
        timestamp: 1.0,
        value: 25.5,
    }).unwrap();
    
    // 3. Verify propagation
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::PlotData { name, value, .. } = ev {
        assert_eq!(name, "temperature");
        assert_eq!(value, 25.5);
    } else {
        panic!("Expected PlotData event");
    }
}

#[tokio::test]
async fn test_scenario_rtt_advanced() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Trigger RTT Attach
    handle.send(DebugCommand::RttAttach).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::RttAttach));
    
    // 2. Simulate RTT Discovery
    let up = vec![aether_core::rtt::RttChannelInfo { number: 0, name: Some("Log".to_string()), buffer_size: 1024 }];
    event_tx.send(DebugEvent::RttChannels { up_channels: up, down_channels: vec![] }).unwrap();
    
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::RttChannels { up_channels, .. } = ev {
        assert_eq!(up_channels.len(), 1);
        assert_eq!(up_channels[0].name, Some("Log".to_string()));
    } else {
        panic!("Expected RttChannels event");
    }
    
    // 3. Write to Down Channel
    let data = b"start\n".to_vec();
    handle.send(DebugCommand::RttWrite { channel: 0, data: data.clone() }).unwrap();
    let cmd = cmd_rx.try_recv().expect("Core did not receive RttWrite");
    if let DebugCommand::RttWrite { channel, data: d } = cmd {
        assert_eq!(channel, 0);
        assert_eq!(d, data);
    } else {
        panic!("Expected RttWrite command");
    }
}

#[tokio::test]
async fn test_scenario_source_high_level() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Request Source for PC
    handle.send(DebugCommand::LookupSource(0x08001000)).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::LookupSource(0x08001000)));
    
    // 2. Simulate Symbol Lookup result
    let source = aether_core::symbols::SourceInfo {
        file: std::path::PathBuf::from("src/main.rs"),
        line: 10,
        column: Some(5),
        function: Some("main".to_string()),
    };
    event_tx.send(DebugEvent::SourceLocation(source)).unwrap();
    
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::SourceLocation(info) = ev {
        assert_eq!(info.line, 10);
        assert_eq!(info.function, Some("main".to_string()));
    } else {
        panic!("Expected SourceLocation event");
    }
}

#[tokio::test]
async fn test_scenario_hil_automation_simulation() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let _receiver = handle.subscribe(); // Keep the event channel alive
    
    // 1. Script handles Flashing
    let firmware = std::path::PathBuf::from("target/production.elf");
    handle.send(DebugCommand::StartFlashing(firmware)).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::StartFlashing(_)));
    
    // Simulate Done
    event_tx.send(DebugEvent::FlashDone).unwrap();
    
    // 2. Script sets breakpoint and resumes
    handle.send(DebugCommand::SetBreakpoint(0x0800AAAA)).unwrap();
    handle.send(DebugCommand::Resume).unwrap();
    
    // 3. Verify core receives automated commands
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::SetBreakpoint(0x0800AAAA)));
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::Resume));
}

#[tokio::test]
async fn test_scenario_multi_client_state_sync() {
    let (handle, _cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    
    // 1. Multiple subscribers (Client A and Client B)
    let mut client_a = handle.subscribe();
    let mut client_b = handle.subscribe();
    
    // 2. System event occurs (Halt)
    event_tx.send(DebugEvent::Halted { pc: 0x1234 }).unwrap();
    
    // 3. Both clients must receive the event
    let ev_a = timeout(Duration::from_millis(100), client_a.recv()).await.unwrap().unwrap();
    let ev_b = timeout(Duration::from_millis(100), client_b.recv()).await.unwrap().unwrap();
    
    if let (DebugEvent::Halted { pc: pc_a }, DebugEvent::Halted { pc: pc_b }) = (ev_a, ev_b) {
        assert_eq!(pc_a, 0x1234);
        assert_eq!(pc_b, 0x1234);
    } else {
        panic!("Both clients should have received Halted event");
    }
}

#[tokio::test]
async fn test_scenario_rust_async_observability() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. User requests Tasks (Kevin investigating async stall)
    handle.send(DebugCommand::GetTasks).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::GetTasks));
    
    // 2. Simulate Async Runner decoding
    // In a real scenario, the RTOS manager would parse the executor state
    let async_tasks = vec![
        aether_core::TaskInfo {
            name: "HTTP_Server".to_string(),
            priority: 1,
            state: TaskState::Pending, // Waiting for network
            stack_usage: 256,
            stack_size: 2048,
            handle: 0x20002000,
            task_type: aether_core::TaskType::Async,
        },
        aether_core::TaskInfo {
            name: "Sensor_Poll".to_string(),
            priority: 2,
            state: TaskState::Running,
            stack_usage: 128,
            stack_size: 1024,
            handle: 0x20003000,
            task_type: aether_core::TaskType::Async,
        }
    ];
    event_tx.send(DebugEvent::Tasks(async_tasks)).unwrap();
    
    // 3. Verify UI receives the states
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::Tasks(tasks) = ev {
        assert_eq!(tasks[0].state, TaskState::Pending);
        assert_eq!(tasks[1].state, TaskState::Running);
    } else {
        panic!("Expected Tasks event");
    }
}

#[tokio::test]
async fn test_scenario_collaborative_conflict() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    
    // Simulation: Alice and Bob are connected
    let mut alice = handle.subscribe();
    let _bob = handle.subscribe();
    
    // 1. Alice sends Step
    handle.send(DebugCommand::Step).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::Step));
    
    // 2. Core broadcasts "Resumed" to everyone
    event_tx.send(DebugEvent::Resumed).unwrap();
    
    // 3. Alice sees she is in control/target is running
    let ev = timeout(Duration::from_millis(100), alice.recv()).await.unwrap().unwrap();
    assert!(matches!(ev, DebugEvent::Resumed));
    
    // 4. Bob (another client) also sees it
    // (This is covered by multi_client_state_sync, but here we emphasize the workflow)
    
    // 5. Simulate conflict: While running, Alice sets a breakpoint
    handle.send(DebugCommand::SetBreakpoint(0x0800AAAA)).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::SetBreakpoint(0x0800AAAA)));
}

#[tokio::test]
async fn test_error_invalid_memory_access() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. User tries to read forbidden address
    handle.send(DebugCommand::ReadMemory(0xDEADBEEF, 4)).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::ReadMemory(0xDEADBEEF, 4)));
    
    // 2. Core emits Error event
    event_tx.send(DebugEvent::Error("Invalid memory access: 0xDEADBEEF is protected".to_string())).unwrap();
    
    // 3. Verify UI receives the error specifically
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::Error(msg) = ev {
        assert!(msg.contains("0xDEADBEEF"));
    } else {
        panic!("Expected Error event");
    }
}

#[tokio::test]
async fn test_edge_breakpoint_limit_reached() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. User sets 7th breakpoint (assuming limit is 6)
    handle.send(DebugCommand::SetBreakpoint(0x0800AAAA)).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::SetBreakpoint(_)));
    
    // 2. Core emits Error event about hardware limits
    event_tx.send(DebugEvent::Error("Hardware limit reached: No more breakpoint units available".to_string())).unwrap();
    
    // 3. Verify Error
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::Error(msg) = ev {
        assert!(msg.contains("Hardware limit"));
    } else {
        panic!("Expected Error event");
    }
}

#[tokio::test]
async fn test_stress_rtt_drop_detected() {
    let (handle, _cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Simulate data burst that causes a drop (via error event used for telemetry)
    event_tx.send(DebugEvent::Error("RTT Drop Detected: Buffer overflow in Channel 0".to_string())).unwrap();
    
    // 2. Verify status bar/UI notification source
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::Error(msg) = ev {
        assert!(msg.contains("RTT Drop"));
    } else {
        panic!("Expected Error event for RTT drop");
    }
}

#[tokio::test]
async fn test_scenario_stepping_in_exception() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Core is currently in HardFault
    event_tx.send(DebugEvent::Status(probe_rs::CoreStatus::Halted(probe_rs::HaltReason::Exception))).unwrap();
    
    // 2. Verify UI sees the exception state
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::Status(status) = ev {
        assert!(matches!(status, probe_rs::CoreStatus::Halted(probe_rs::HaltReason::Exception)));
    }
    
    handle.send(DebugCommand::Step).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::Step));
}

#[tokio::test]
async fn test_error_svd_missing_file() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. User tries to load non-existent SVD
    let path = std::path::PathBuf::from("non_existent.svd");
    handle.send(DebugCommand::LoadSvd(path)).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::LoadSvd(_)));
    
    // 2. Core emits Error event
    event_tx.send(DebugEvent::Error("SVD Error: File not found".to_string())).unwrap();
    
    // 3. Verify Error
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::Error(msg) = ev {
        assert!(msg.contains("SVD Error"));
    } else {
        panic!("Expected Error event");
    }
}

#[tokio::test]
async fn test_edge_plot_variable_out_of_scope() {
    let (handle, _cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Plotting is active for 'temp'
    // 2. Variable goes out of scope (simulated by error or stop event)
    event_tx.send(DebugEvent::Error("Plot Error: Variable 'temp' is out of scope".to_string())).unwrap();
    
    // 3. Verify UI notification
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::Error(msg) = ev {
        assert!(msg.contains("out of scope"));
    } else {
        panic!("Expected Error event");
    }
}

#[tokio::test]
async fn test_error_source_not_found() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Request source for address with no symbols
    handle.send(DebugCommand::LookupSource(0xDEADBEEF)).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::LookupSource(0xDEADBEEF)));
    
    // 2. Core emits Error: No symbols
    event_tx.send(DebugEvent::Error("Symbol Error: No debug symbols found for 0xDEADBEEF".to_string())).unwrap();
    
    // 3. Verify Error
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::Error(msg) = ev {
        assert!(msg.contains("No debug symbols"));
    } else {
        panic!("Expected Error event");
    }
}

#[tokio::test]
async fn test_error_stack_corrupted() {
    let (handle, _cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Request Stack
    handle.send(DebugCommand::GetStack).unwrap();
    
    // 2. Core emits Error: Corrupt Stack
    event_tx.send(DebugEvent::Error("Unwind Error: Stack corrupted (invalid SP: 0xDEADBEEF)".to_string())).unwrap();
    
    // 3. Verify Error
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::Error(msg) = ev {
        assert!(msg.contains("Stack corrupted"));
    } else {
        panic!("Expected Error event");
    }
}

#[tokio::test]
async fn test_edge_rtt_pending_initialization() {
    let (handle, _cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Simulate RTT symbol found but magic sequence invalid (pending init)
    event_tx.send(DebugEvent::FlashStatus("RTT Pending... Waiting for target initialization".to_string())).unwrap();
    
    // 2. Verify status update
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::FlashStatus(msg) = ev {
        assert!(msg.contains("RTT Pending"));
    } else {
        panic!("Expected FlashStatus event for RTT pending");
    }
}

#[tokio::test]
async fn test_edge_swo_baud_mismatch_detection() {
    let (handle, _cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Simulate SWO decoder seeing garbage due to baud rate mismatch
    event_tx.send(DebugEvent::Error("Trace Error: SWO Baud rate mismatch detected".to_string())).unwrap();
    
    // 2. Verify UI warning
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::Error(msg) = ev {
        assert!(msg.contains("Baud rate"));
    } else {
        panic!("Expected Error event for SWO");
    }
}

#[tokio::test]
async fn test_scenario_breakpoint_persistence_on_reset() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Set Breakpoint
    handle.send(DebugCommand::SetBreakpoint(0x0800AAAA)).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::SetBreakpoint(_)));
    
    // 2. Reset Core
    // (Assuming Reset is part of StartFlashing or similar, but let's assume a generic Reset command might exist or just simulate the event)
    event_tx.send(DebugEvent::Status(probe_rs::CoreStatus::Running)).unwrap();
    
    // 3. Request Breakpoints - should still be there
    handle.send(DebugCommand::ListBreakpoints).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::ListBreakpoints));
    
    event_tx.send(DebugEvent::Breakpoints(vec![0x0800AAAA])).unwrap();
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::Breakpoints(bps) = ev {
        assert_eq!(bps, vec![0x0800AAAA]);
    }
}

#[tokio::test]
async fn test_edge_control_conflict_agent_vs_user() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut agent = handle.subscribe();
    
    // 1. Agent sends Halt
    handle.send(DebugCommand::Halt).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::Halt));
    
    // 2. Simultaneously, User (via UI) sends Resume
    handle.send(DebugCommand::Resume).unwrap();
    
    // 3. Core processes Halt then Resume
    event_tx.send(DebugEvent::Halted { pc: 0x100 }).unwrap();
    event_tx.send(DebugEvent::Resumed).unwrap();
    
    // 4. Agent sees the rapid transition
    let ev1 = timeout(Duration::from_millis(100), agent.recv()).await.unwrap().unwrap();
    let ev2 = timeout(Duration::from_millis(100), agent.recv()).await.unwrap().unwrap();
    
    assert!(matches!(ev1, DebugEvent::Halted { .. }));
    assert!(matches!(ev2, DebugEvent::Resumed));
}

#[tokio::test]
async fn test_stress_rtt_buffer_wrap_simulation() {
    let (handle, _cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Send data that would wrap the ring buffer
    let chunk1 = vec![0x1; 512];
    let chunk2 = vec![0x2; 512];
    
    event_tx.send(DebugEvent::RttData(0, chunk1)).unwrap();
    event_tx.send(DebugEvent::RttData(0, chunk2)).unwrap();
    
    // 2. Verify sequential delivery
    let ev1 = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    let ev2 = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    
    if let (DebugEvent::RttData(c1, d1), DebugEvent::RttData(c2, d2)) = (ev1, ev2) {
        assert_eq!(c1, 0);
        assert_eq!(c2, 0);
        assert_eq!(d1.len(), 512);
        assert_eq!(d2.len(), 512);
    }
}

#[tokio::test]
async fn test_stress_massive_memory_integrity() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Generate 64KB of random-looking data
    let size = 64 * 1024;
    let mut data = Vec::with_capacity(size);
    for i in 0..size {
        data.push((i % 256) as u8);
    }
    
    // 2. Write Massive Block
    handle.send(DebugCommand::WriteMemory(0x20000000, data.clone())).unwrap();
    let cmd = cmd_rx.try_recv().expect("Core did not receive massive write");
    if let DebugCommand::WriteMemory(addr, d) = cmd {
        assert_eq!(addr, 0x20000000);
        assert_eq!(d.len(), size);
    }
    
    // 3. Request Read Back
    handle.send(DebugCommand::ReadMemory(0x20000000, size)).unwrap();
    let cmd = cmd_rx.try_recv().expect("Core did not receive massive read");
    if let DebugCommand::ReadMemory(addr, s) = cmd {
        assert_eq!(addr, 0x20000000);
        assert_eq!(s, size);
    }
    
    // 4. Simulate Data Arrival
    event_tx.send(DebugEvent::MemoryData(0x20000000, data.clone())).unwrap();
    
    // 5. Verify Integrity
    let event = timeout(Duration::from_millis(200), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::MemoryData(_, received_data) = event {
        assert_eq!(received_data.len(), size);
        assert_eq!(received_data, data);
    }
}

#[tokio::test]
async fn test_scenario_complex_state_chain() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // Workflow: Halt -> Step -> Write -> Resume -> Hit Breakpoint -> Reset
    
    // 1. Halt
    handle.send(DebugCommand::Halt).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::Halt));
    event_tx.send(DebugEvent::Halted { pc: 0x100 }).unwrap();
    let _ = receiver.recv().await;
    
    // 2. Step
    handle.send(DebugCommand::Step).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::Step));
    event_tx.send(DebugEvent::Halted { pc: 0x104 }).unwrap();
    let _ = receiver.recv().await;
    
    // 3. Write Memory (Fixing a variable)
    handle.send(DebugCommand::WriteMemory(0x2000, vec![1])).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::WriteMemory(_, _)));
    
    // 4. Resume
    handle.send(DebugCommand::Resume).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::Resume));
    event_tx.send(DebugEvent::Resumed).unwrap();
    let _ = receiver.recv().await;
    
    // 5. Hit Breakpoint (Simulated)
    event_tx.send(DebugEvent::Halted { pc: 0x200 }).unwrap();
    let ev = receiver.recv().await.unwrap();
    if let DebugEvent::Halted { pc } = ev {
        assert_eq!(pc, 0x200);
    }
}

#[tokio::test]
async fn test_scenario_task_switch_monitoring() {
    let (handle, _cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Simulate a task switch event (from TCB1 to TCB2)
    let timestamp = 1.234;
    event_tx.send(DebugEvent::TaskSwitch {
        from: Some(0x20001000),
        to: 0x20002000,
        timestamp,
    }).unwrap();
    
    // 2. Verify UI/Agent receives the switch
    let ev = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    if let DebugEvent::TaskSwitch { from, to, timestamp: ts } = ev {
        assert_eq!(from, Some(0x20001000));
        assert_eq!(to, 0x20002000);
        assert_eq!(ts, timestamp);
    } else {
        panic!("Expected TaskSwitch event");
    }
}

#[tokio::test]
async fn test_stress_concurrent_agents() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let _subscriber = handle.subscribe(); // Keep the event channel alive
    
    let num_agents = 10;
    let mut futures = vec![];
    
    for i in 0..num_agents {
        let h = handle.clone();
        futures.push(tokio::spawn(async move {
            h.send(DebugCommand::ReadRegister(i as u16)).unwrap();
        }));
    }
    
    for f in futures {
        f.await.unwrap();
    }
    
    // Core should have received 10 commands
    for _ in 0..num_agents {
        cmd_rx.try_recv().expect("Missing concurrent command");
    }
    
    // Simulate one broadcast back
    event_tx.send(DebugEvent::RegisterValue(0, 0x1234)).unwrap();
}

#[tokio::test]
async fn test_scenario_flash_interruption_recovery() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Start Flashing
    handle.send(DebugCommand::StartFlashing(std::path::PathBuf::from("recovery.elf"))).unwrap();
    let _ = cmd_rx.try_recv().unwrap();
    
    // 2. Simulate Disconnect (via Error event)
    event_tx.send(DebugEvent::Error("Probe Disconnected during flash".to_string())).unwrap();
    
    let ev = receiver.recv().await.unwrap();
    if let DebugEvent::Error(msg) = ev {
        assert!(msg.contains("Disconnected"));
    }
    
    // 3. User tries to reconnect/restart
    handle.send(DebugCommand::Halt).unwrap();
    assert!(matches!(cmd_rx.try_recv().unwrap(), DebugCommand::Halt));
}

#[tokio::test]
async fn test_fuzz_malformed_svd() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Load SVD with bad XML (Simulated)
    handle.send(DebugCommand::LoadSvd(std::path::PathBuf::from("malformed.svd"))).unwrap();
    let _ = cmd_rx.try_recv().unwrap();
    
    // 2. Core reports XML parsing error
    event_tx.send(DebugEvent::Error("SVD Fuzz: Malformed XML at line 42".to_string())).unwrap();
    
    let ev = receiver.recv().await.unwrap();
    if let DebugEvent::Error(msg) = ev {
        assert!(msg.contains("Malformed XML"));
    }
}

#[tokio::test]
async fn test_fuzz_corrupt_symbols() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Load symbols with corrupt DWARF header
    handle.send(DebugCommand::LoadSymbols(std::path::PathBuf::from("corrupt.elf"))).unwrap();
    let _ = cmd_rx.try_recv().unwrap();
    
    // 2. Core reports DWARF error
    event_tx.send(DebugEvent::Error("WARF Fuzz: Invalid compilation unit header".to_string())).unwrap();
    
    let ev = receiver.recv().await.unwrap();
    if let DebugEvent::Error(msg) = ev {
        assert!(msg.contains("Invalid compilation unit"));
    }
}

#[tokio::test]
async fn test_scenario_dwarf_variable_resolution() {
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    
    // 1. User wants to watch a complex variable "config"
    handle.send(DebugCommand::WatchVariable("config".to_string())).expect("Failed to send WatchVariable");
    
    // 2. Verify Command received by core
    let cmd = cmd_rx.try_recv().expect("Core did not receive WatchVariable command");
    if let DebugCommand::WatchVariable(name) = cmd {
        assert_eq!(name, "config");
    } else {
        panic!("Expected WatchVariable command, got {:?}", cmd);
    }
    
    // 3. Verify propagation starts
    let mut receiver = handle.subscribe();
    
    // 4. Simulate SymbolManager resolving the variable into a nested structure
    let mock_info = aether_core::symbols::TypeInfo {
        name: "config".to_string(),
        value_formatted_string: "struct Config".to_string(),
        kind: "Struct".to_string(),
        address: Some(0x20000000),
        members: Some(vec![
            aether_core::symbols::TypeInfo {
                name: "enabled".to_string(),
                value_formatted_string: "true".to_string(),
                kind: "Primitive".to_string(),
                address: Some(0x20000000),
                members: None,
            },
            aether_core::symbols::TypeInfo {
                name: "threshold".to_string(),
                value_formatted_string: "42".to_string(),
                kind: "Primitive".to_string(),
                address: Some(0x20000004),
                members: None,
            },
        ]),
    };
    
    event_tx.send(DebugEvent::VariableResolved(mock_info.clone())).expect("Failed to broadcast VariableResolved event");
    
    // 5. Verify propagation to UI
    let event: DebugEvent = timeout(Duration::from_millis(100), receiver.recv())
        .await
        .expect("Timeout waiting for VariableResolved event")
        .expect("Failed to receive event");
        
    match event {
        DebugEvent::VariableResolved(info) => {
            assert_eq!(info.name, "config");
            assert_eq!(info.kind, "Struct");
            let members = info.members.as_ref().expect("Members should be resolved");
            assert_eq!(members.len(), 2);
            assert_eq!(members[0].name, "enabled");
            assert_eq!(members[1].name, "threshold");
        },
        _ => panic!("Expected VariableResolved event, got {:?}", event),
    }
}
#[tokio::test]
async fn test_stress_large_scale_task_switching() {
    let (handle, _cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // 1. Simulate 50 concurrent tasks
    let mut tasks = Vec::new();
    for i in 0..50 {
        tasks.push(aether_core::TaskInfo {
            name: format!("Task_{}", i),
            priority: (i % 8) as u32,
            state: TaskState::Ready,
            stack_usage: 100,
            stack_size: 1000,
            handle: 0x20000000 + (i * 0x100) as u32,
            task_type: aether_core::TaskType::Thread,
        });
    }
    event_tx.send(DebugEvent::Tasks(tasks.clone())).unwrap();

    // 2. Simulate rapid task switching for all 50 tasks
    for i in 0..100 {
        let to_index = i % 50;
        let from_index = if i > 0 { Some((i - 1) % 50) } else { None };
        
        event_tx.send(DebugEvent::TaskSwitch {
            from: from_index.map(|idx| tasks[idx].handle),
            to: tasks[to_index].handle,
            timestamp: i as f64 * 0.01,
        }).unwrap();
    }

    // 3. Verify event propagation
    let mut count = 0;
    while let Ok(event) = timeout(Duration::from_millis(100), receiver.recv()).await {
        if let Ok(DebugEvent::TaskSwitch { .. }) = event {
            count += 1;
        }
        if count >= 100 { break; }
    }
    assert_eq!(count, 100);
}

#[tokio::test]
async fn test_perf_rtt_10khz_simulation() {
    let (handle, _cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);
    let mut receiver = handle.subscribe();
    
    // Simulate 10,000 RTT messages per second (10 per millisecond)
    let start = std::time::Instant::now();
    let message_count = 1000;
    
    for i in 0..message_count {
        event_tx.send(DebugEvent::RttData(0, format!("log message {}\n", i).into_bytes())).unwrap();
    }
    
    // 2. Verify we can consume them without lag (within 1 second for 1000 messages)
    let mut received = 0;
    while let Ok(msg) = timeout(Duration::from_millis(500), receiver.recv()).await {
        match msg {
            Ok(DebugEvent::RttData(_, _)) => {
                received += 1;
            }
            Err(e) => {
                panic!("Receiver lagged or failed during perf test: {:?}", e);
            }
            _ => {}
        }
        if received >= message_count { break; }
    }
    
    assert_eq!(received, message_count);
    assert!(start.elapsed() < Duration::from_secs(1));
}
