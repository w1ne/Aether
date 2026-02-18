use probe_rs::probe::list::Lister;

fn main() {
    let lister = Lister::new();
    let probes = lister.list_all();
    println!("Found {} probes:", probes.len());
    for (i, info) in probes.iter().enumerate() {
        println!(
            "{}: {} (VID: {:04X}, PID: {:04X}, Serial: {:?})",
            i, info.identifier, info.vendor_id, info.product_id, info.serial_number
        );
    }
}
