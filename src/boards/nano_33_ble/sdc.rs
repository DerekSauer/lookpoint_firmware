// SPDX-FileCopyrightText: 2025 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Softdevice Controller developed by Nordic Semiconductor provides a
//! certified Bluetooth Low Energy controller for their MCUs.
//!
//! This is only one half of a fully functioning BLE stack and implements the
//! Link Layer. This is a low-level real time protocol which, using the radio
//! (physical layer), transmits and receives over the air communications.
//!
//! The other half of the stack is the host which provides higher level
//! functionality such as advertising and the GATT server.
//!
//! This split between responsibilities makes it simpler to use the same host
//! with different controller implementations.
//!
//! The softdevice will take exclusive ownership of the following peripherals:
//!
//! - ECB
//! - CCM
//! - AAR
//! - PPI Channels 17, 18, 20 to 29
//!
//! Ownership of the  ECB, CCM, and AAR peripherals cannot be enforced at
//! compile time so we must ensure they are not used directly elsewhere in the
//! application.
//!
//! nRF's documentation for the Softdevice is available at:
//! https://docs.nordicsemi.com/bundle/ncs-latest/page/nrfxlib/softdevice_controller/README.html

use embassy_nrf::{Peri, peripherals};
use nrf_mpsl::MultiprotocolServiceLayer;
use nrf_sdc::SoftdeviceController;
use static_cell::StaticCell;

use super::rng_driver::RngDriver;

/// Initialize the Softdevice BLE controller.
///
/// # Panic
///
/// The Softdevice is critical to the device's function. Failure to initialize
/// the Softdevice will cause this function to panic.
#[allow(clippy::too_many_arguments)]
pub fn init_ble_controller<'sdc, 'mpsl: 'sdc, 'rng: 'sdc>(
    ppi_ch17: Peri<'static, peripherals::PPI_CH17>,
    ppi_ch18: Peri<'static, peripherals::PPI_CH18>,
    ppi_ch20: Peri<'static, peripherals::PPI_CH20>,
    ppi_ch21: Peri<'static, peripherals::PPI_CH21>,
    ppi_ch22: Peri<'static, peripherals::PPI_CH22>,
    ppi_ch23: Peri<'static, peripherals::PPI_CH23>,
    ppi_ch24: Peri<'static, peripherals::PPI_CH24>,
    ppi_ch25: Peri<'static, peripherals::PPI_CH25>,
    ppi_ch26: Peri<'static, peripherals::PPI_CH26>,
    ppi_ch27: Peri<'static, peripherals::PPI_CH27>,
    ppi_ch28: Peri<'static, peripherals::PPI_CH28>,
    ppi_ch29: Peri<'static, peripherals::PPI_CH29>,
    mpsl: &'mpsl MultiprotocolServiceLayer<'_>,
    rng_driver: &'rng mut RngDriver,
) -> SoftdeviceController<'sdc> {
    let peripherals = nrf_sdc::Peripherals::new(
        ppi_ch17, ppi_ch18, ppi_ch20, ppi_ch21, ppi_ch22, ppi_ch23, ppi_ch24, ppi_ch25, ppi_ch26,
        ppi_ch27, ppi_ch28, ppi_ch29,
    );

    // The BLE controller reserves some memory for its internal state.
    // The amount of memory needed depends on which controller features are enabled
    // and how many connections it is configured to serve. A warning will be issued
    // if the amount requested is too low or too high.
    let memory = {
        static MEM: StaticCell<nrf_sdc::Mem<1432>> = StaticCell::new();
        MEM.init_with(|| {
            let mem = nrf_sdc::Mem::new();
            defmt::info!(
                "[controller] memory reserved: {} bytes",
                core::mem::size_of_val(&mem)
            );
            mem
        })
    };

    match build_softdevice(peripherals, rng_driver, memory, mpsl) {
        Ok(sdc) => {
            defmt::info!("[controller] initialized");
            sdc
        }
        Err(error) => {
            defmt::panic!(
                "[controller] unable to initialize, vendor error code: {}",
                error
            );
        }
    }
}

/// Convenience function to construct a [`SoftdeviceController`] with simple
/// error return.
fn build_softdevice<'a, const N: usize>(
    softdevice_peripherals: nrf_sdc::Peripherals<'a>,
    softdevice_rng_driver: &'a mut RngDriver,
    softdevice_memory: &'a mut nrf_sdc::Mem<N>,
    mpsl: &'a MultiprotocolServiceLayer,
) -> Result<SoftdeviceController<'a>, nrf_sdc::Error> {
    nrf_sdc::Builder::new()?
        .support_adv()?
        .support_peripheral()?
        .peripheral_count(1)?
        .build(
            softdevice_peripherals,
            softdevice_rng_driver,
            mpsl,
            softdevice_memory,
        )
}
