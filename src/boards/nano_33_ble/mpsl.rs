// SPDX-FileCopyrightText: 2025 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Multiprotocol Service Layer (MPSL) is a library that provides services
//! common to the many communication protocols supported by the nRF52's radio.
//!
//! The MPLS provides a timeslot API that may be used to share access to the
//! radio and encryption services. This allow different communication protocols
//! to coexist on the same radio.
//!
//! Writing to flash and reading chip temperature block the CPU until they
//! complete and will interfere radio functionlity. The MPSL allows flash and
//! temperature operations to be scheduled when the radio is not in use.
//!
//! The MPSL will take exclusive ownership of the following peripherals:
//!
//! - RTC0
//! - TIMER0
//! - TEMP
//! - NVMC
//! - PPI Channels 19, 30, 31
//!
//! The MPSL binds to the following interrupts:
//!
//! - EGU0_SWI0 (Low priority handler)
//! - RADIO (High priority handler)
//! - CLOCK_POWER (High priority handler)
//! - RTC0 (High priority handler)
//! - TIMER0 (High priority handler)
//!
//! nRF's documentation for the MPSL is available at:
//! https://docs.nordicsemi.com/bundle/ncs-latest/page/nrfxlib/mpsl/README.html

use embassy_nrf::{Peri, peripherals};
use nrf_mpsl::SessionMem;
use nrf_sdc::mpsl::{self, MultiprotocolServiceLayer};
use static_cell::StaticCell;

/// Number of timeslots the Service Layer will make available to the
/// application. Two slots is sufficient for flash and temperature operations.
const NUM_TIMESLOTS: usize = 2;

/// Task that will run the MPSL's event loop. Started when the MPSL is
/// initialized.
#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static mpsl::MultiprotocolServiceLayer<'static>) -> ! {
    defmt::info!("[mpsl] event loop task started");
    mpsl.run().await;
}

/// Initialize the Multiprotocol Service Layer.
///
/// # Panic
///
/// The MPSL is critical to the device's function. Failure to initialize the
/// MPSL will cause this function to panic.
#[allow(clippy::too_many_arguments)]
pub fn init_service_layer<'mpsl>(
    rtc0: Peri<'static, peripherals::RTC0>,
    timer0: Peri<'static, peripherals::TIMER0>,
    temp: Peri<'static, peripherals::TEMP>,
    ppi_ch19: Peri<'static, peripherals::PPI_CH19>,
    ppi_ch30: Peri<'static, peripherals::PPI_CH30>,
    ppi_ch31: Peri<'static, peripherals::PPI_CH31>,
    task_spawner: &embassy_executor::Spawner,
) -> &'mpsl MultiprotocolServiceLayer<'static> {
    let peripherals = mpsl::Peripherals::new(rtc0, timer0, temp, ppi_ch19, ppi_ch30, ppi_ch31);

    // Map hardware interrupts to interrupt handlers provided by the MPSL.
    embassy_nrf::bind_interrupts!(
        struct MpslIrqs {
            EGU0_SWI0 => mpsl::LowPrioInterruptHandler;
            CLOCK_POWER => mpsl::ClockInterruptHandler;
            RADIO => mpsl::HighPrioInterruptHandler;
            TIMER0 => mpsl::HighPrioInterruptHandler;
            RTC0 => mpsl::HighPrioInterruptHandler;
        }
    );

    // The Nano 33 BLE has an external oscillator.
    let clock_config = mpsl::raw::mpsl_clock_lfclk_cfg_t {
        source:                  mpsl::raw::MPSL_CLOCK_LF_SRC_XTAL as u8,
        rc_ctiv:                 0,
        rc_temp_ctiv:            0,
        accuracy_ppm:            50,
        skip_wait_lfclk_started: false,
    };

    // The MPSL reserves some memory for its internal state.
    let memory = {
        static MEM: StaticCell<SessionMem<NUM_TIMESLOTS>> = StaticCell::new();
        MEM.init_with(|| {
            let session_mem = SessionMem::new();
            defmt::info!(
                "[mpsl] session memory reserved: {} bytes",
                core::mem::size_of_val(&session_mem)
            );
            session_mem
        })
    };

    // The MPSL will remain persistent for the entire run time of the device.
    let mpsl = {
        static MPSL: StaticCell<MultiprotocolServiceLayer> = StaticCell::new();
        MPSL.init_with(|| {
            match MultiprotocolServiceLayer::with_timeslots(
                peripherals,
                MpslIrqs,
                clock_config,
                memory,
            ) {
                Ok(mpsl) => {
                    defmt::info!("[mpsl] initialized");
                    mpsl
                }
                Err(error) => {
                    defmt::panic!("[mpsl] unable to initialize, error code: {}", error);
                }
            }
        })
    };

    // The task that pumps the MPSL's event loop will run forever.
    task_spawner.must_spawn(mpsl_task(mpsl));

    mpsl
}
