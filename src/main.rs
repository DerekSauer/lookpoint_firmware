// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only

#![no_main]
#![no_std]

// Install a panic handler outputting to probe-rs.
use panic_probe as _;

// Install `defmt` as the global logger.
use defmt_rtt as _;

// Running on a Nordic Semiconductor nRF52840.
use nrf52840_hal as _;

#[cortex_m_rt::entry]
fn main() -> ! {
    loop {
        defmt::info!("Tick!");
        cortex_m::asm::delay(3_276_800);
    }
}
