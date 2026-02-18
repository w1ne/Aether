use probe_rs::probe::list::Lister;
use probe_rs::Permissions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lister = Lister::new();
    let probes = lister.list_all();
    if probes.is_empty() {
        println!("No probes found");
        return Ok(());
    }

    let mut probe = probes[0].open()?;
    println!("Opened probe: {}", probes[0].identifier);

    println!("Selecting SWD protocol...");
    probe.select_protocol(probe_rs::probe::WireProtocol::Swd)?;

    println!("Attempting attach with TargetSelector::Auto...");
    use probe_rs::config::TargetSelector;
    match probe.attach(TargetSelector::Auto, Permissions::default()) {
        Ok(s) => println!("SUCCESS: Auto-detected chip: {}", s.target().name),
        Err(e) => println!("Attach with TargetSelector::Auto failed: {}", e),
    }

    Ok(())
}
