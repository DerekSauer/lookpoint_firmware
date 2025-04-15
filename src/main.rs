// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only
//
#![feature(impl_trait_in_assoc_type)]
#![cfg_attr(debug_assertions, allow(dead_code, unused_variables))]
#![no_main]
#![no_std]

mod ble;

use {defmt_rtt as _, panic_probe as _};

/// Entry point.
#[embassy_executor::main]
async fn main(task_spawner: embassy_executor::Spawner) {
    defmt::info!("Device is starting up.");

    let peripherals = init_peripherals();

    let (softdevice, mpsl) = ble::controller::initialize_ble_controller(
        nrf_sdc::mpsl::Peripherals::new(
            peripherals.RTC0,
            peripherals.TIMER0,
            peripherals.TEMP,
            peripherals.PPI_CH19,
            peripherals.PPI_CH30,
            peripherals.PPI_CH31,
        ),
        nrf_sdc::Peripherals::new(
            peripherals.PPI_CH17,
            peripherals.PPI_CH18,
            peripherals.PPI_CH20,
            peripherals.PPI_CH21,
            peripherals.PPI_CH22,
            peripherals.PPI_CH23,
            peripherals.PPI_CH24,
            peripherals.PPI_CH25,
            peripherals.PPI_CH26,
            peripherals.PPI_CH27,
            peripherals.PPI_CH28,
            peripherals.PPI_CH29,
        ),
        peripherals.RNG,
    );
}

/// Initialize the MCU, its peripherals, and interrupts.
fn init_peripherals() -> embassy_nrf::Peripherals {
    use embassy_nrf::config;

    let mut nrf_config = config::Config::default();

    // Our board has an external 32Mhz oscillator.
    nrf_config.hfclk_source = config::HfclkSource::ExternalXtal;
    nrf_config.lfclk_source = config::LfclkSource::ExternalXtal;

    defmt::info!("Microcontroller initialized.");

    embassy_nrf::init(nrf_config)
}
