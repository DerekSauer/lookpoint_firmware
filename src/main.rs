// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only
//
#![feature(impl_trait_in_assoc_type)]
#![cfg_attr(debug_assertions, allow(dead_code, unused_variables))]
#![no_main]
#![no_std]

mod bluetooth;

use bluetooth::device_name::DeviceName;
use bluetooth::server::BluetoothServer;
use {defmt_rtt as _, panic_probe as _};

/// Entry point.
#[embassy_executor::main]
async fn main(task_spawner: embassy_executor::Spawner) {
    defmt::info!("Device is starting up.");

    let peripherals = init_peripherals();

    let device_name = DeviceName::from("Lookpoint Tracker");
    let bluetooth_server = BluetoothServer::new(&device_name, 2, task_spawner);
}

/// Initialize the MCU, its peripherals, and interrupts.
fn init_peripherals() -> embassy_nrf::Peripherals {
    use embassy_nrf::{config, interrupt};

    let mut nrf_config = config::Config::default();

    // Our board has an external 32Mhz oscillator.
    nrf_config.hfclk_source = config::HfclkSource::ExternalXtal;
    nrf_config.lfclk_source = config::LfclkSource::ExternalXtal;

    // The Bluetooth Softdevice reserves interrupt priorities 0, 1, and 4.
    nrf_config.time_interrupt_priority = interrupt::Priority::P2;
    nrf_config.gpiote_interrupt_priority = interrupt::Priority::P3;

    defmt::info!("Microcontroller initialized.");

    embassy_nrf::init(nrf_config)
}
