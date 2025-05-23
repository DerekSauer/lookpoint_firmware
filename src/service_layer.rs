// SPDX-FileCopyrightText: 2025 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only

//! The Multiprotocol Service Layer (MPSL) is a library that provides services
//! common to the many communication protocols supported by the nRF52's radio.
//!
//! The MPLS provides a timeslot API that may be used to share access to the
//! radio and encryption services. This allow different communication protocols
//! to coexist on the same radio.
//!
//! Writing to flash and reading chip temperature block the CPU until they
//! complete and will interfere radio functionlity. The MPSL allows flash and
//! temperature operations to be schedule when the radio is not in use.
//!
//! The MPSL will take exclusive ownership of the following peripherals:
//!
//! - RTC0
//! - TIMER0
//! - TEMP
//! - NVMC
//! - PPI Channels 19, 30, 31
//!
//! The MPSL enables the following interrupts:
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
use nrf_sdc::mpsl::{self, MultiprotocolServiceLayer, SessionMem};
use static_cell::StaticCell;

/// Number of timeslots the MPSL will make available to the application.
const MPSL_TIMESLOTS: usize = 2;

/// Storage for the MPSL's timeslot sessions.
static MPSL_SESSION_MEM: StaticCell<SessionMem<MPSL_TIMESLOTS>> = StaticCell::new();

/// Storage for the MPSL itself.
static MPSL: StaticCell<MultiprotocolServiceLayer> = StaticCell::new();

/// Wrapper around the Multiprotocol Service Layer (MPSL).
/// Provides safe access to Flash reading & writing, temperature readings, and
/// the ECB encryption module without interfering with radio operation.
pub struct ServiceLayer<'mpsl> {
    /// Reference to the MPSL stored in static memory.
    mpsl: &'mpsl MultiprotocolServiceLayer<'static>,

    /// Flash driver that schedules write operations using the
    /// MPSL's timeslot API to avoid blocking radio operations.
    flash_driver: mpsl::Flash<'static>,
}

impl ServiceLayer<'_> {
    /// Start the multiprotocol service layer (MPSL).
    /// The MPSL will take exclusive ownership of the passed in peripherals and
    /// the MPSL's event loop will be started immediately after initialization.
    ///
    /// # Panic
    ///
    /// Initializing the MPSL is critical to the application. This function will
    /// panic if the MPSL cannot be initialized. The panic message will include
    /// an error code that may be referenced in nRF's documentation.
    pub fn new(
        rtc0: Peri<'static, peripherals::RTC0>,
        timer0: Peri<'static, peripherals::TIMER0>,
        temp: Peri<'static, peripherals::TEMP>,
        nvmc: Peri<'static, peripherals::NVMC>,
        ppi_ch19: Peri<'static, peripherals::PPI_CH19>,
        ppi_ch30: Peri<'static, peripherals::PPI_CH30>,
        ppi_ch31: Peri<'static, peripherals::PPI_CH31>,
        task_spawner: embassy_executor::Spawner,
    ) -> Self {
        // Our board has an external crystal oscillator.
        let mpsl_clock_config = mpsl::raw::mpsl_clock_lfclk_cfg_t {
            source:                  mpsl::raw::MPSL_CLOCK_LF_SRC_XTAL as u8,
            rc_ctiv:                 0,
            rc_temp_ctiv:            0,
            accuracy_ppm:            50,
            skip_wait_lfclk_started: false,
        };

        // Interrupts bound to interrupt handlers provided by the MPSL.
        embassy_nrf::bind_interrupts!(
            struct MPSLIrqs {
                EGU0_SWI0 => mpsl::LowPrioInterruptHandler;
                CLOCK_POWER => mpsl::ClockInterruptHandler;
                RADIO => mpsl::HighPrioInterruptHandler;
                TIMER0 => mpsl::HighPrioInterruptHandler;
                RTC0 => mpsl::HighPrioInterruptHandler;
            }
        );

        // These peripherals are taken by the MPSL to ensure at compile time that it has
        // exclusive access to them.
        let mpsl_peripherals =
            mpsl::Peripherals::new(rtc0, timer0, temp, ppi_ch19, ppi_ch30, ppi_ch31);

        // This reference will be dropped at the end of this function, but the MPSL
        // retains a pointer to the static location in memory.
        let mpsl_session_mem = MPSL_SESSION_MEM.init_with(|| SessionMem::new());

        // The MPSL itself lives in static memory. We'll retain a reference to it.
        let mpsl = MPSL.init_with(|| {
            match MultiprotocolServiceLayer::with_timeslots(
                mpsl_peripherals,
                MPSLIrqs,
                mpsl_clock_config,
                mpsl_session_mem,
            ) {
                Ok(mpsl) => mpsl,
                Err(err) => {
                    panic!("[mpsl] failed to initialize service layer: {:?}", err)
                }
            }
        });

        defmt::info!("[mpsl] service layer initialized");

        // The flash driver will take ownership of the NVMC peripheral.
        let flash_driver = mpsl::Flash::take(mpsl, nvmc);

        // Run the background task that pumps the MPSL's event loop.
        task_spawner.must_spawn(mpsl_task(mpsl));

        defmt::info!("[mpsl] service layer event loop started");

        Self { mpsl, flash_driver }
    }

    /// Get a reference to the underlying MPSL.
    pub fn get_mpsl_ref(&self) -> &MultiprotocolServiceLayer<'static> {
        self.mpsl
    }

    /// Measure the temperature on chip, to nearest °C.
    /// This is a blocking operation that can take 50us plus the delay until
    /// next free timeslot where the radio is not performing an operation.
    pub fn get_temperature(&self) -> i32 {
        self.mpsl.get_temperature().degrees()
    }

    /// Get a reference to the MPSL's flash driver.
    /// This driver implements the `embedded_storage` traits.
    pub fn get_flash<'mpsl>(&'mpsl self) -> &'mpsl mpsl::Flash<'static> {
        &self.flash_driver
    }

    /// Get a mutable reference to the MPSL's flash driver.
    /// This driver implements the `embedded_storage` traits.
    pub fn get_flash_mut<'mpsl>(&'mpsl mut self) -> &'mpsl mut mpsl::Flash<'static> {
        &mut self.flash_driver
    }
}

/// Embassy task that will run the multiprotocol service layer's event loop.
#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static mpsl::MultiprotocolServiceLayer<'static>) -> ! {
    mpsl.run().await;
}
