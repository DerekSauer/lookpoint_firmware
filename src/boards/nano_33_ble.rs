// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Board support module for the Arduino Nano 33 BLE.

mod mpsl;
mod rng_driver;
mod sdc;

use embassy_nrf::config::{Config, Debug, HfclkSource, LfclkSource};
use embassy_nrf::interrupt::Priority;
use nrf_sdc::SoftdeviceController;

use crate::boards::nano_33_ble::rng_driver::RngDriver;

/// Board support for the Arduino Nano 33 BLE.
pub struct Board<'mpsl, 'rng> {
    /// Multiprotocol service layer.
    service_layer: &'mpsl nrf_sdc::mpsl::MultiprotocolServiceLayer<'static>,

    /// Random number generator.
    pub rng_driver: RngDriver<'rng>,
}

impl<'mpsl, 'sdc, 'rng> Board<'mpsl, 'rng> {
    /// Initialize the board and its peripherals.
    /// The BLE controller remains uninitialized.
    pub fn init(task_spawner: &embassy_executor::Spawner) -> Self {
        let mut board_config = Config::default();

        // This board has external oscillators for the high and low frequency clocks.
        board_config.hfclk_source = HfclkSource::ExternalXtal;
        board_config.lfclk_source = LfclkSource::ExternalXtal;

        // The SoftDevice BLE controller reserves interrupt priorities 0, 1, and 4.
        // Move Embassy's interrupts to unused priority levels.
        board_config.time_interrupt_priority = Priority::P2;
        board_config.gpiote_interrupt_priority = Priority::P2;

        // I want folks to be able to hack on this device.
        board_config.debug = Debug::Allowed;

        let peripherals = embassy_nrf::init(board_config);

        let service_layer = mpsl::init_service_layer(
            peripherals.RTC0,
            peripherals.TIMER0,
            peripherals.TEMP,
            peripherals.PPI_CH19,
            peripherals.PPI_CH30,
            peripherals.PPI_CH31,
            task_spawner,
        );

        let rng_driver = RngDriver::new(peripherals.RNG);

        Self {
            service_layer,
            rng_driver,
        }
    }

    /// Initialize the BLE controller.
    pub fn get_ble_controller(&'mpsl mut self) -> SoftdeviceController<'sdc>
    where
        'mpsl: 'sdc,
        'rng: 'sdc,
    {
        // SAFETY: As `self` is initialized, peripherals are safe to use and the BLE
        // controller is the only device using these PPI channels.
        let peripherals = unsafe { embassy_nrf::Peripherals::steal() };

        sdc::init_ble_controller(
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
            self.service_layer,
            &mut self.rng_driver,
        )
    }
}
