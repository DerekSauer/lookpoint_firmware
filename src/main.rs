// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only
//
#![feature(impl_trait_in_assoc_type)]
#![feature(round_char_boundary)]
#![cfg_attr(debug_assertions, allow(dead_code, unused_variables))]
#![no_main]
#![no_std]

mod ble;

use {defmt_rtt as _, panic_probe as _};

/// Entry point.
#[embassy_executor::main]
async fn main(task_spawner: embassy_executor::Spawner) {
    defmt::info!("[mcu] device is starting up");

    let peripherals = init_peripherals();

    let (ble_controller, mpsl) = ble::controller::initialize_ble_controller(
        task_spawner,
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

    let ficr = embassy_nrf::pac::FICR;
    let high = u64::from(ficr.deviceid(1).read());
    let addr = high << 32 | u64::from(ficr.deviceid(0).read());
    let addr = addr | 0x0000_c000_0000_0000;
    let addr: [u8; 6] = addr.to_le_bytes()[..6].try_into().unwrap();

    ble::gatt_server::run(
        "Lookpoint Tracker".into(),
        addr,
        task_spawner,
        ble_controller,
    )
    .await;
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
