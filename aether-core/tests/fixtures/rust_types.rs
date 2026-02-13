pub fn main() {
    let _v: Vec<u32> = vec![1, 2, 3];
    let _o: Option<u32> = Some(42);
    let _r: Result<u32, &str> = Ok(100);
    
    // Globals to make them easier to find via symbol lookups in tests
    unsafe {
        G_V = vec![1, 2, 3];
        G_O = Some(42);
        G_R = Ok(100);
    }
}

#[no_mangle]
pub static mut G_V: Vec<u32> = Vec::new();
#[no_mangle]
pub static mut G_O: Option<u32> = None;
#[no_mangle]
pub static mut G_R: Result<u32, &str> = Err("error");
