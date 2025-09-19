// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-or-later

use core::cell::RefCell;

use embassy_nrf::mode::Blocking;
use embassy_nrf::peripherals::RNG;
use embassy_nrf::rng::Rng;
use embassy_nrf::{Peri, peripherals, rng};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;

/// Random number generator that can be sent between tasks.
pub struct RngDriver<'rng> {
    /// Underlying random number generation driver.
    embassy_rng_driver: Mutex<NoopRawMutex, RefCell<rng::Rng<'rng, peripherals::RNG, Blocking>>>,
}

impl RngDriver<'_> {
    pub fn new(rng_peripheral: Peri<'static, RNG>) -> Self {
        let rng_driver = Rng::new_blocking(rng_peripheral);

        Self {
            embassy_rng_driver: Mutex::new(RefCell::new(rng_driver)),
        }
    }
}

impl rand_core_06::CryptoRng for RngDriver<'_> {}
impl rand_core_06::RngCore for RngDriver<'_> {
    fn next_u32(&mut self) -> u32 {
        self.embassy_rng_driver
            .lock(|inner| inner.borrow_mut().blocking_next_u32())
    }

    fn next_u64(&mut self) -> u64 {
        self.embassy_rng_driver
            .lock(|inner| inner.borrow_mut().next_u64())
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.embassy_rng_driver
            .lock(|inner| inner.borrow_mut().blocking_fill_bytes(dest))
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core_06::Error> {
        self.embassy_rng_driver
            .lock(|inner| inner.borrow_mut().try_fill_bytes(dest))
    }
}

impl rand_core_09::CryptoRng for RngDriver<'_> {}
impl rand_core_09::RngCore for RngDriver<'_> {
    fn next_u32(&mut self) -> u32 {
        self.embassy_rng_driver
            .lock(|inner| inner.borrow_mut().blocking_next_u32())
    }

    fn next_u64(&mut self) -> u64 {
        self.embassy_rng_driver
            .lock(|inner| inner.borrow_mut().next_u64())
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.embassy_rng_driver
            .lock(|inner| inner.borrow_mut().blocking_fill_bytes(dest))
    }
}
