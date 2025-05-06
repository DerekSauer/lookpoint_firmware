// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only
//
#![feature(impl_trait_in_assoc_type)]
#![feature(round_char_boundary)]
#![cfg_attr(debug_assertions, allow(dead_code, unused_variables))]
#![no_main]
#![no_std]

mod service_layer;

use {defmt_rtt as _, panic_probe as _};

/// Entry point.
#[embassy_executor::main]
async fn main(task_spawner: embassy_executor::Spawner) {
    defmt::info!("[mcu] device is starting up");

    let peripherals = init_peripherals();

    let service_layer = service_layer::ServiceLayer::new(
        peripherals.RTC0,
        peripherals.TIMER0,
        peripherals.TEMP,
        peripherals.NVMC,
        peripherals.PPI_CH19,
        peripherals.PPI_CH30,
        peripherals.PPI_CH31,
        task_spawner,
    );
}

/// Initialize the MCU, its peripherals, and interrupts.
fn init_peripherals() -> embassy_nrf::Peripherals {
    use embassy_nrf::config;
    use embassy_nrf::interrupt::Priority;

    let mut nrf_config = config::Config::default();

    // Our board has an external 32Mhz oscillator.
    nrf_config.hfclk_source = config::HfclkSource::ExternalXtal;
    nrf_config.lfclk_source = config::LfclkSource::ExternalXtal;

    // The SoftDevice BLE controller reserves interrupt priorities 0, 1, and 4.
    // Move embassy's interrupts to unused priority levels.
    nrf_config.gpiote_interrupt_priority = Priority::P2;
    nrf_config.time_interrupt_priority = Priority::P2;

    defmt::info!("[mcu] microcontroller initialized");

    embassy_nrf::init(nrf_config)
}
