// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only

#![cfg_attr(feature = "nightly", feature(impl_trait_in_assoc_type))]
#![no_main]
#![no_std]

use defmt_rtt as _;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_time::{Duration, Timer};
use panic_probe as _;

#[embassy_executor::task]
async fn blink_led(mut led: Output<'static>, interval: Duration) {
    loop {
        defmt::info!("LED on!");
        led.set_high();
        Timer::after(interval).await;

        defmt::info!("LED off!");
        led.set_low();
        Timer::after(interval).await;
    }
}

#[embassy_executor::main]
async fn main(task_spawner: embassy_executor::Spawner) {
    let peripherals = embassy_nrf::init(Default::default());

    let led = Output::new(peripherals.P0_13, Level::Low, OutputDrive::Standard);
    task_spawner
        .spawn(blink_led(led, Duration::from_millis(1000)))
        .expect("The MCU is on fire.");
}
