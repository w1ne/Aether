#![allow(missing_docs)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::significant_drop_tightening)]
use aether_agent_api::run_server;
use aether_agent_api::proto::aether_debug_client::AetherDebugClient;
use aether_agent_api::proto::Empty;
use aether_core::{SessionHandle, DebugCommand, DebugEvent};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_agent_api_basic_ops() {
    // 1. Setup mock session handle
    let (handle, cmd_rx, event_tx) = SessionHandle::new_test();
    let handle = Arc::new(handle);

    // 2. Start server on random port
    let port = 50055; // For test simplicity, ideally find a free port
    let server_handle = handle.clone();
    tokio::spawn(async move {
        if let Err(e) = run_server(server_handle, "127.0.0.1", port).await {
            eprintln!("Test server error: {}", e);
        }
    });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // 3. Connect client
    let addr = format!("http://127.0.0.1:{}", port);
    let mut client = AetherDebugClient::connect(addr).await.expect("Failed to connect");

    // 4. Test Subscribe
    let mut stream = client.subscribe_events(Empty {}).await.expect("Subscribe failed").into_inner();

    // 5. Test Command transmission
    client.halt(Empty {}).await.expect("Halt failed");

    // Verify command reached core
    let cmd = cmd_rx.try_recv().expect("No command received in core");
    match cmd {
        DebugCommand::Halt => {}
        _ => panic!("Expected Halt command, got {:?}", cmd),
    }

    // 6. Test Event transmission
    let pc_val = 0x12345678;
    event_tx.send(DebugEvent::Halted { pc: pc_val }).expect("Failed to send event");

    // Verify event reached client
    let event = stream.message().await.expect("Stream error").expect("No event received in client");
    match event.event {
        Some(aether_agent_api::proto::debug_event::Event::Halted(h)) => {
            assert_eq!(h.pc, pc_val);
        }
        _ => panic!("Expected Halted event, got {:?}", event),
    }
}
