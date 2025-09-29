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
//! compile time so we must ensure they are not used elsewhere in the
//! application.
//!
//! nRF's documentation for the Softdevice is available at:
//! https://docs.nordicsemi.com/bundle/ncs-latest/page/nrfxlib/softdevice_controller/README.html

use embassy_nrf::mode::{self, Async};
use embassy_nrf::rng::{InterruptHandler, Rng};
use embassy_nrf::{Peri, bind_interrupts, peripherals};
use nrf_mpsl::MultiprotocolServiceLayer;
use nrf_sdc::SoftdeviceController;
use rand_chacha::ChaChaRng;
use rand_core::SeedableRng;
use static_cell::StaticCell;
use trouble_host::Stack;
use trouble_host::prelude::DefaultPacketPool;

use crate::ble::BleResources;

/// Amount of memory needed by the Softdevice.
const SDC_MEM: usize = 1432;

/// Initialize the BLE controller and host.
#[allow(clippy::too_many_arguments)]
pub fn init_ble_stack<'stack>(
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
    rng: Peri<'static, peripherals::RNG>,
    mpsl: &'static nrf_sdc::mpsl::MultiprotocolServiceLayer<'static>,
    address: trouble_host::Address,
) -> Stack<'stack, SoftdeviceController<'static>, DefaultPacketPool> {
    let softdevice_peripherals = nrf_sdc::Peripherals::new(
        ppi_ch17, ppi_ch18, ppi_ch20, ppi_ch21, ppi_ch22, ppi_ch23, ppi_ch24, ppi_ch25, ppi_ch26,
        ppi_ch27, ppi_ch28, ppi_ch29,
    );

    bind_interrupts!(struct RngIrq {
        RNG => InterruptHandler<peripherals::RNG>;
    });

    // Statically store the BLE controller's random number generator to
    // simplify lifetime constraints.
    let mut controller_rng = {
        static RNG: StaticCell<Rng<'static, peripherals::RNG, mode::Async>> = StaticCell::new();
        RNG.init_with(|| Rng::new(rng, RngIrq))
    };

    // The BLE controller will own the RNG peripheral. Use it to seed another random
    // number generator for the host. The host only uses this to seed its internal
    // RNG and this RNG will drop at end of function.
    let mut host_rng = match ChaChaRng::from_rng(&mut controller_rng) {
        Ok(rng) => rng,
        Err(_) => {
            defmt::panic!("[ble] failed to initialize the BLE host's random number generator",)
        }
    };

    // The Softdevice BLE controller reserves some memory for its own state.
    // Will panic if not enough memory is provided. A log message will be emitted
    // indicating the correct amount.
    let controller_memory = {
        static SDC_MEMORY: StaticCell<nrf_sdc::Mem<SDC_MEM>> = StaticCell::new();
        SDC_MEMORY.init_with(|| {
            let mem = nrf_sdc::Mem::new();
            defmt::info!(
                "[sdc] Softdevice memory reserved: {} bytes",
                core::mem::size_of_val(&mem)
            );
            mem
        })
    };

    let controller = match build_softdevice(
        softdevice_peripherals,
        controller_rng,
        controller_memory,
        mpsl,
    ) {
        Ok(sdc) => {
            defmt::info!("[sdc] Softdevice BLE controller initialized");
            sdc
        }
        Err(error) => {
            defmt::panic!(
                "[sdc] failed to initialize the Softdevice BLE controller, error code: {}",
                error
            )
        }
    };

    // Memory reserved for the BLE host's internal state.
    let host_resources = {
        static HOST_RESOURCES: StaticCell<BleResources> = StaticCell::new();
        HOST_RESOURCES.init_with(BleResources::new)
    };

    trouble_host::new(controller, host_resources)
        .set_random_address(address)
        .set_random_generator_seed(&mut host_rng)
}

/// Convenience function to construct a [`SoftdeviceController`] with simple
/// error return.
pub fn build_softdevice<'a>(
    softdevice_peripherals: nrf_sdc::Peripherals<'a>,
    rng_driver: &'a mut Rng<'a, peripherals::RNG, Async>,
    softdevice_memory: &'a mut nrf_sdc::Mem<SDC_MEM>,
    mpsl: &'a MultiprotocolServiceLayer,
) -> Result<SoftdeviceController<'a>, nrf_sdc::Error> {
    nrf_sdc::Builder::new()?
        .support_adv()?
        .support_peripheral()?
        .support_dle_peripheral()?
        .support_phy_update_peripheral()?
        .support_le_2m_phy()?
        .peripheral_count(1)?
        .build(softdevice_peripherals, rng_driver, mpsl, softdevice_memory)
}
