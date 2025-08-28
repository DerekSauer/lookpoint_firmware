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

use core::marker::Send;

use embassy_nrf::{Peri, peripherals};
use nrf_mpsl::MultiprotocolServiceLayer;
use nrf_sdc::SoftdeviceController;
use rand_core::{CryptoRng, RngCore};
use static_cell::StaticCell;

use crate::service_layer::ServiceLayer;

// Memory reserved for the Softdevice. The amount of memory needed will
// differ depending on which Softdevice features are enabled. A warning
// containing the amount needed will be logged. Panic if amount is too low.
static SOFTDEVICE_MEM: StaticCell<nrf_sdc::Mem<1432>> = StaticCell::new();

/// Initialize the Softdevice BLE controller.
///
/// # Panic
///
/// Initialization of the Softdevice is critical to the application. This
/// function will panic if the Softdevice cannot be initialized. The panic
/// message will include an error code that may be referenced in nRF's
/// documentation.
pub fn initialize_ble_controller<'softdevice, RngDriver: CryptoRng + RngCore + Send>(
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
    service_layer: &'softdevice ServiceLayer,
    rng_driver: &'softdevice mut RngDriver,
    task_spawner: embassy_executor::Spawner,
) -> SoftdeviceController<'softdevice> {
    // This reference will be dropped at the end of this function, but the
    // Softdevice retains a pointer to the static location in memory.
    let softdevice_memory = SOFTDEVICE_MEM.init_with(|| nrf_sdc::Mem::new());

    let softdevice_peripherals = nrf_sdc::Peripherals::new(
        ppi_ch17, ppi_ch18, ppi_ch20, ppi_ch21, ppi_ch22, ppi_ch23, ppi_ch24, ppi_ch25, ppi_ch26,
        ppi_ch27, ppi_ch28, ppi_ch29,
    );

    let softdevice = match build_softdevice(
        softdevice_peripherals,
        rng_driver,
        softdevice_memory,
        service_layer.get_mpsl_ref(),
    ) {
        Ok(initialized_softdevice) => {
            defmt::info!("[ble] Softdevice controller initialized");
            initialized_softdevice
        }
        Err(error) => {
            panic!(
                "[ble] failed to initialize Softdevice controller with error: {}",
                error
            )
        }
    };

    softdevice
}

/// Convenience function to construct a `SoftdeviceController` with simple error
/// return.
fn build_softdevice<'a, const N: usize, RngDriver: CryptoRng + RngCore + Send>(
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
