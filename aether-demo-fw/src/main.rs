#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    // LD2 is on PA5 on Nucleo-L476RG
    let mut led = Output::new(p.PA5, Level::Low, Speed::Low);

    let mut counter: u32 = 0;

    loop {
        led.set_high();
        Timer::after_millis(500).await;

        led.set_low();
        Timer::after_millis(500).await;

        counter = counter.wrapping_add(1);
        defmt::info!("Blink count: {}", counter);

        // Dummy function call to test stack trace
        do_work(counter);
    }
}

#[inline(never)]
fn do_work(val: u32) {
    // Just a dummy computation to have a function frame
    let _x = val * 2;
    core::hint::black_box(_x);
}
