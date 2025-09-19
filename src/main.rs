// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(debug_assertions, allow(dead_code, unused_variables))]
#![no_main]
#![no_std]

mod boards;

use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(task_spawner: embassy_executor::Spawner) {
    let mut board = boards::Board::init(&task_spawner);
    let ble_controller = board.get_ble_controller();
}
