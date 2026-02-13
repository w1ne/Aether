#[allow(dead_code)]
pub struct Deep {
    pub a: u32,
    pub b: f32,
}

#[allow(dead_code)]
pub struct Nested {
    pub x: i32,
    pub b: bool,
    pub deep: Deep,
}

#[allow(dead_code)]
pub struct Config {
    pub enabled: bool,
    pub threshold: u32,
    pub nested: Nested,
}

#[no_mangle]
pub static mut MY_CONFIG: Config = Config {
    enabled: true,
    threshold: 42,
    nested: Nested { 
        x: 10, 
        b: false,
        deep: Deep { a: 100, b: 3.14 }
    },
};

fn main() {
    unsafe {
        MY_CONFIG.threshold += 1;
    }
}
