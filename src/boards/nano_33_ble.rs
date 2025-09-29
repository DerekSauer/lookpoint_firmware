// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Board support module for the Arduino Nano 33 BLE (Rev2).
//!
//! Vendor's documentation available at:
//! https://docs.arduino.cc/hardware/nano-33-ble-rev2/

mod mpsl;
mod sdc;

use embassy_executor::Spawner;
use embassy_nrf::config::{Config, Debug, HfclkSource, LfclkSource};
use embassy_nrf::interrupt::Priority;
use nrf_mpsl::MultiprotocolServiceLayer;
use nrf_sdc::SoftdeviceController;
use nrf_sdc::mpsl::Flash;
use static_cell::StaticCell;
use trouble_host::prelude::DefaultPacketPool;
use trouble_host::{Address, Host, Stack};

/// Board support for the Arduino Nano 33 BLE (Rev2).
pub struct Board<'mpsl, 'sdc> {
    /// Reference to the MPSL's location in static memory.
    mpsl: &'mpsl MultiprotocolServiceLayer<'static>,

    /// Flash storage handler.
    flash: Flash<'static>,

    /// BLE stack (Controller & host resources).
    ble_stack: Stack<'sdc, SoftdeviceController<'mpsl>, DefaultPacketPool>,
}

impl<'mpsl, 'sdc> Board<'mpsl, 'sdc> {
    /// Initialize the [`Board`], its peripherals, and the BLE stack.
    pub fn init(task_spawner: &Spawner) -> Self {
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

        // Initialize the MPSL and start its event loop task which will run forever.
        let mpsl = {
            static MPSL: StaticCell<MultiprotocolServiceLayer> = StaticCell::new();
            MPSL.init_with(|| {
                mpsl::init_service_layer(
                    peripherals.RTC0,
                    peripherals.TIMER0,
                    peripherals.TEMP,
                    peripherals.PPI_CH19,
                    peripherals.PPI_CH30,
                    peripherals.PPI_CH31,
                )
            })
        };
        task_spawner.must_spawn(mpsl::mpsl_task(mpsl));

        // The MPSL offers a flash storage interface that schedules reads &
        // writes to not conflict with the radio.
        let flash = Flash::take(mpsl, peripherals.NVMC);

        let ble_address = Self::get_ble_address();
        let ble_stack = sdc::init_ble_stack(
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
            peripherals.RNG,
            mpsl,
            ble_address,
        );

        Self {
            mpsl,
            flash,
            ble_stack,
        }
    }

    /// Returns the BLE [`Host`] of this [`Board`].
    pub fn get_ble_host(&'sdc self) -> Host<'sdc, SoftdeviceController<'mpsl>, DefaultPacketPool> {
        self.ble_stack.build()
    }

    /// Retrieve the MAC address of this [`Board`].
    // TODO: Ensure the returned address matches the QR Code on the MCU.
    fn get_ble_address() -> Address {
        // The manufacturer of the board has burned a unique MAC address to the
        // board's Factory Information Configuration Registers (FICR).
        let ficr = embassy_nrf::pac::FICR;

        // Lower 16 bits of DEVICEADDR[1] contain the most significant bits of the MAC.
        let msb = u64::from(ficr.deviceaddr(1).read() & 0x0000ffff);

        // DEVICEADDR[0] contains the least significant bits of the MAC.
        let lsb = u64::from(ficr.deviceaddr(0).read());

        // Shift the `msb` over by 32-bits and append the `lsb`.
        let address = msb << 32 | lsb;

        // UNWRAP: Infallible. Taking lower 6 bytes from an 8 byte value.
        Address::random(address.to_le_bytes()[0..6].try_into().unwrap())
    }
}
