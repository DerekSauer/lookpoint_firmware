// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(debug_assertions, allow(dead_code, unused_variables))]
#![no_main]
#![no_std]

mod ble_controller;
mod service_layer;

use embassy_nrf::rng;
use rand_chacha::rand_core::SeedableRng;
use {defmt_rtt as _, panic_probe as _};

// TODO: Move this into a `board` module.
// The RNG interrupt is triggered when new random numbers are written to the
// RNG's `value` register.
embassy_nrf::bind_interrupts!(
    struct RngIrq {
        RNG => embassy_nrf::rng::InterruptHandler<embassy_nrf::peripherals::RNG>;
    }
);

/// Entry point.
#[embassy_executor::main]
async fn main(task_spawner: embassy_executor::Spawner) {
    defmt::info!("[mcu] device is starting up");

    let peripherals = init_peripherals();

    // The Softdevice BLE controller will take a mutable reference to the RNG
    // driver. The driver will take ownership of the RNG peripheral.
    // Before handing over the driver to the Softdevice we'll use it to seed another
    // random number generator for the BLE host.
    let mut rng_driver = rng::Rng::new(peripherals.RNG, RngIrq);
    let rng_chacha = rand_chacha::ChaCha12Rng::from_rng(&mut rng_driver);

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

    let softdevice = ble_controller::initialize_ble_controller(
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
        &service_layer,
        &mut rng_driver,
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
