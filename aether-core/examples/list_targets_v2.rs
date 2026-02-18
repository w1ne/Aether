fn main() {
    let families = probe_rs_target::families().expect("Failed to get families");
    for family in families {
        if family.name.to_lowercase().contains("l476") {
            println!("Family: {}", family.name);
            for variant in family.variants {
                 println!("  - {}", variant.name);
            }
        }
    }
}
