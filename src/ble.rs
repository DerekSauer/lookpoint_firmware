// SPDX-FileCopyrightText: 2025 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-or-later

use trouble_host::prelude::*;

pub mod advertise;
pub mod gatt_server;
pub mod services;

/// This device can service only one connection.
const MAX_CONNECTIONS: usize = 1;

/// This device will advertise the same data each advertising window, so
/// multiple advertising sets are not needed.
const MAX_ADVERTISING_SETS: usize = 1;

/// Two channels will be required for L2CAP transfers (Signal + ATT).
const MAX_L2CAP_CHANNELS: usize = 2;

pub type BleResources =
    HostResources<DefaultPacketPool, MAX_CONNECTIONS, MAX_L2CAP_CHANNELS, MAX_ADVERTISING_SETS>;

/// Background task that pumps the BLE stack's event loop.
///
/// This task must be run alongside other BLE tasks. Recommend joining it with
/// the advertising task.
///
/// # Panic
///
/// Any errors that occur in the BLE event loop are likely unrecoverable and
/// will result in a panic.
pub async fn ble_background_task<C: Controller, P: PacketPool>(runner: &mut Runner<'_, C, P>) {
    if let Err(error) = runner.run().await {
        match error {
            BleHostError::Controller(_) => {
                defmt::panic!("[ble_task] error occured in the BLE controller.")
            }
            BleHostError::BleHost(host_error) => {
                defmt::panic!("[ble_task] error occured in the BLE host: {}", host_error)
            }
        }
    }
}
