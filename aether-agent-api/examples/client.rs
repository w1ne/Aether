use aether_agent_api::proto::aether_debug_client::AetherDebugClient;
use aether_agent_api::proto::Empty;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = AetherDebugClient::connect("http://[::1]:50051").await?;

    println!("Connected to Aether Agent API");

    // Get Status
    let status = client.get_status(Empty {}).await?;
    println!("Status: {:?}", status.into_inner());

    // Subscribe to events
    let mut stream = client.subscribe_events(Empty {}).await?.into_inner();

    tokio::spawn(async move {
        while let Some(event) = stream.message().await.unwrap() {
            println!("Event received: {:?}", event);
        }
    });

    // Send Halt
    println!("Sending Halt...");
    client.halt(Empty {}).await?;
    
    // Wait a bit
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Send Resume
    println!("Sending Resume...");
    client.resume(Empty {}).await?;

    Ok(())
}
