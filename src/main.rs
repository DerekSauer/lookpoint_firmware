// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only

#![cfg_attr(debug_assertions, allow(dead_code, unused_variables))]
#![cfg_attr(feature = "nightly", feature(impl_trait_in_assoc_type))]
#![no_main]
#![no_std]

mod bluetooth;

use bluetooth::bluetooth_device::BluetoothDevice;
use defmt_rtt as _;
use embassy_nrf as _;
use panic_probe as _;

/// Maximum number of connections the Bluetooth device can manage.
pub const MAX_BLE_CONNECTIONS: u8 = 1;

/// Name of this device.
pub const DEVICE_NAME: &str = "Lookpoint Tracker";

/// Initialize MCU peripherals and configure interrupts.
fn init_peripherals() {
    let mut nrf_config = embassy_nrf::config::Config::default();

    // Our board has an external 32Mhz oscillator.
    nrf_config.hfclk_source = embassy_nrf::config::HfclkSource::ExternalXtal;
    nrf_config.lfclk_source = embassy_nrf::config::LfclkSource::ExternalXtal;

    // The Bluetooth Softdevice reserves interrupt priorities 0, 1, and 4.
    nrf_config.time_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    nrf_config.gpiote_interrupt_priority = embassy_nrf::interrupt::Priority::P3;

    let _peripherals = embassy_nrf::init(nrf_config);

    defmt::info!("Microcontroller peripherals initialized.");
}

/// Real entry point of the application.
async fn inner_main(task_spawner: embassy_executor::Spawner) {
    defmt::info!("Device is starting up.");

    init_peripherals();

    let bluetooth_device =
        BluetoothDevice::new(DEVICE_NAME, MAX_BLE_CONNECTIONS).run(&task_spawner);
}

/// Stub entry point.
#[embassy_executor::main]
async fn main(task_spawner: embassy_executor::Spawner) {
    // While debugging over RTT we cannot set breakpoints in the main function or
    // RTT will not be intialized when main is suspended during a break.
    inner_main(task_spawner).await;
}
