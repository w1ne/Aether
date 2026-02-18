use aether_agent_api::proto::aether_debug_client::AetherDebugClient;
use aether_agent_api::proto::{Empty, ReadMemoryRequest, WatchVariableRequest};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Starting Agent API Verification ---");
    
    let url = "http://127.0.0.1:50051";
    println!("Connecting to {}...", url);
    let mut client = AetherDebugClient::connect(url).await?;
    
    // 1. Subscribe to events
    println!("[AGENT] Subscribing to events...");
    let mut stream = client.subscribe_events(Empty {}).await?.into_inner();
    
    // 2. Perform actions
    println!("[AGENT] Sending Reset...");
    client.reset(Empty {}).await?;
    
    println!("[AGENT] Sending Watch variable 'counter'...");
    client.watch_variable(WatchVariableRequest { name: "counter".to_string() }).await?;
    
    println!("[AGENT] Sending Halt...");
    client.halt(Empty {}).await?;
    
    println!("[AGENT] Reading memory at 0x08000000...");
    let resp = client.read_memory(ReadMemoryRequest { address: 0x08000000, length: 16 }).await?.into_inner();
    println!("[AGENT] Received {} bytes of memory data.", resp.data.len());
    
    // 3. Verify events (non-blocking check for a few events)
    println!("[AGENT] Checking event stream for expected updates...");
    let mut event_count = 0;
    while let Ok(Some(event)) = tokio::time::timeout(tokio::time::Duration::from_millis(500), stream.next()).await {
        println!("[AGENT] Received Event: {:?}", event);
        event_count += 1;
        if event_count >= 3 { break; }
    }
    
    println!("--- Agent API Verification Completed Successfully ---");
    Ok(())
}
