// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(debug_assertions, allow(dead_code, unused_variables))]
#![no_main]
#![no_std]

mod ble;
mod boards;

use {defmt_rtt as _, panic_probe as _};

use crate::ble::advertise::advertise_task;
use crate::ble::ble_background_task;
use crate::ble::gatt_server::GattServer;
use crate::boards::Board;

/// Device name advertised over BLE.
static ADV_NAME: &str = "Lookpoint Tracker";

#[embassy_executor::main]
async fn main(task_spawner: embassy_executor::Spawner) {
    let board = Board::init(&task_spawner);

    let mut host = board.get_ble_host();

    let gatt_server = match GattServer::start(ADV_NAME) {
        Ok(gatt_server) => gatt_server,
        Err(error) => defmt::panic!("[gatt] failed to start the GATT server: {}", error),
    };

    // Main loop
    embassy_futures::join::join(
        ble_background_task(&mut host.runner),
        advertise_task(ADV_NAME, &mut host.peripheral, &gatt_server),
    )
    .await;
}
