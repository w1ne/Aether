use probe_rs::config::Target;

fn main() {
    let targets = probe_rs::config::get_targets().expect("Failed to get targets");
    // targets is likely an iterator or vec of strings?
    // In 0.12+ it was a bit different. Let's try to just search for the string.
}
