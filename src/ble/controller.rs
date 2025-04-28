// SPDX-FileCopyrightText: 2025 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only

use embassy_nrf::{Peri, peripherals, rng};
use mpsl::MultiprotocolServiceLayer;
use nrf_sdc::{SoftdeviceController, mpsl};
use static_cell::StaticCell;

/// Clock configuration for the Multiprotocol Service Layer.
type MpslClockConfig = mpsl::raw::mpsl_clock_lfclk_cfg_t;
const MPSL_CLOCK_CONFIG: MpslClockConfig = MpslClockConfig {
    // Our board has an external crystal oscillator.
    source: mpsl::raw::MPSL_CLOCK_LF_SRC_XTAL as u8,

    // The external crystal handles its own calibration.
    rc_ctiv:      0,
    rc_temp_ctiv: 0,

    // Our crystal's datasheet claims 50 ppm accuracy at temperatures the device is expected to
    // operate (0 to 40 celcius).
    accuracy_ppm: 50,

    // Wait until the low frequency clock is started before initializing the MPSL.
    skip_wait_lfclk_started: false,
};

// Hardware interrupt bound to the random number generator peripheral.
embassy_nrf::bind_interrupts!(
    struct RngIrq {
        RNG => embassy_nrf::rng::InterruptHandler<peripherals::RNG>;
    }
);

// Hardware interrupts bound to handlers provided by the MPSL.
embassy_nrf::bind_interrupts!(
    struct MPSLIrqs {
        EGU0_SWI0 => mpsl::LowPrioInterruptHandler;
        CLOCK_POWER => mpsl::ClockInterruptHandler;
        RADIO => mpsl::HighPrioInterruptHandler;
        TIMER0 => mpsl::HighPrioInterruptHandler;
        RTC0 => mpsl::HighPrioInterruptHandler;
    }
);

/// Initialize the Bluetooth Low Energy controller. This is one half of the host
/// controller interface providing low level management of the radio and
/// associated resources.
///
/// The Multiprotocol Service Layer and Softdevice will take ownership of
/// numerous MCU peripherals passed in through their relevant parameters.
///
/// # Panic
///
/// Failure to initialize the Multiprotocol Service Layer or Softdevice is fatal
/// and the application will panic with an error code. The error code is derived
/// from the Softdevice's C API.
pub fn initialize_ble_controller<'a>(
    task_spawner: embassy_executor::Spawner,
    mpsl_peripherals: nrf_sdc::mpsl::Peripherals<'a>,
    softdevice_peripherals: nrf_sdc::Peripherals<'a>,
    rng_peripheral: Peri<'a, peripherals::RNG>,
) -> (
    SoftdeviceController<'a>,
    &'static MultiprotocolServiceLayer<'a>,
) {
    // The BLE Softdevice takes control of numerous system resources,
    // peripherals, and interrupts. The MPSL provides an interface where
    // the application can schedule a time to access these resources in a manner
    // that does not conflict with the Softdevice's use of these resources or the
    // radio.
    let mpsl = {
        static MPSL: StaticCell<MultiprotocolServiceLayer> = StaticCell::new();
        MPSL.init_with(|| {
            match MultiprotocolServiceLayer::new(mpsl_peripherals, MPSLIrqs, MPSL_CLOCK_CONFIG) {
                Ok(initialized_mpsl) => {
                    defmt::info!("[ble] multiprotocol service layer initialized");
                    initialized_mpsl
                }
                Err(error) => panic!(
                    "[ble] failed to initialize multiprotocol service layer with error: {}",
                    error
                ),
            }
        })
    };

    // The MPSL runs its own event loop behind the scenes.
    task_spawner.must_spawn(mpsl_task(mpsl));
    defmt::info!("[ble] multiprotocol service layer event loop task started");

    // Random number generation driver used by the Softdevice for cryptographic
    // functions.
    let softdevice_rng_driver = {
        static SOFTDEVICE_RNG_DRIVER: StaticCell<rng::Rng<peripherals::RNG>> = StaticCell::new();
        SOFTDEVICE_RNG_DRIVER.init_with(|| rng::Rng::new(rng_peripheral, RngIrq))
    };

    // RAM reserved for the Softdevice, in bytes. The amount of RAM needed will
    // differ depending on which Softdevice features are enabled. A warning will be
    // logged if the amount is not correct. Will panic if the amount is too low.
    let softdevice_memory = {
        static SOFTDEVICE_MEMORY: StaticCell<nrf_sdc::Mem<1448>> = StaticCell::new();
        SOFTDEVICE_MEMORY.init_with(|| nrf_sdc::Mem::new())
    };

    let softdevice = match build_softdevice(
        softdevice_peripherals,
        softdevice_rng_driver,
        softdevice_memory,
        mpsl,
    ) {
        Ok(initialized_softdevice) => {
            defmt::info!("[ble] controller initialized");
            initialized_softdevice
        }
        Err(error) => {
            panic!(
                "[ble] failed to initialize controller with error: {}",
                error
            )
        }
    };

    (softdevice, mpsl)
}

/// Helper function to construct a `SoftdeviceController` with simple
/// error return.
fn build_softdevice<'a, const N: usize>(
    softdevice_peripherals: nrf_sdc::Peripherals<'a>,
    softdevice_rng_driver: &'a mut rng::Rng<peripherals::RNG>,
    softdevice_memory: &'a mut nrf_sdc::Mem<N>,
    mpsl: &'a MultiprotocolServiceLayer,
) -> Result<SoftdeviceController<'a>, nrf_sdc::Error> {
    nrf_sdc::Builder::new()?
        .support_adv()?
        .support_peripheral()?
        .peripheral_count(super::NUM_PERIPH_CONNECTIONS as u8)?
        .build(
            softdevice_peripherals,
            softdevice_rng_driver,
            mpsl,
            softdevice_memory,
        )
}

/// Task that will run the multiprotocol service layer's event loop.
#[embassy_executor::task]
pub async fn mpsl_task(mpsl: &'static mpsl::MultiprotocolServiceLayer<'static>) -> ! {
    mpsl.run().await;
}
