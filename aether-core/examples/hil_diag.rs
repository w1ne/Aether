use probe_rs::probe::{list::Lister, WireProtocol};
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
    probe.select_protocol(WireProtocol::Swd)?;

    let chip = "STM32L476RGTx";
    println!("Attempting to attach to {}...", chip);
    
    match probe.attach(chip, Permissions::default()) {
        Ok(_session) => {
            println!("SUCCESS: Attached to {} without under-reset!", chip);
            return Ok(());
        }
        Err(e) => {
            println!("Regular attach failed: {}", e);
            println!("Attempting attach under reset...");
            // Re-open since probe might have been moved/consumed or in bad state?
            // Actually let's just try to reopen if we can.
            let mut probe = lister.list_all()[0].open()?;
            probe.select_protocol(WireProtocol::Swd)?;
            match probe.attach_under_reset(chip, Permissions::default()) {
                Ok(_session) => {
                    println!("SUCCESS: Attached to {} under reset!", chip);
                }
                Err(e2) => {
                    println!("Attach under reset failed: {}", e2);
                    
                    println!("Attempting auto-detect...");
                    let mut probe = lister.list_all()[0].open()?;
                    probe.select_protocol(WireProtocol::Swd)?;
                    match probe.attach("auto", Permissions::default()) {
                        Ok(s) => println!("SUCCESS: Auto-detected chip: {}", s.target().name),
                        Err(e3) => println!("Auto-detect failed: {}", e3),
                    }
                }
            }
        }
    }

    Ok(())
}
