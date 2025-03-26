// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only

use core::sync::atomic::AtomicU8;

use embassy_executor::Spawner;
use nrf_softdevice::ble;

use super::bluetooth_device::{BluetoothDevice, Running};
use super::device_name::DeviceName;

/// Bluetooth server managing the Bluetooth device, handling advertising, and
/// exposing services to connected clients.
pub struct BluetoothServer {
    /// Connection to the initialized and running Bluetooth controller.
    bluetooth_device: BluetoothDevice<Running>,

    /// Maximum number of connections the Bluetooth device can service.
    max_connections: u8,

    /// Current number of connections to this Bluetooth device.
    num_connections: AtomicU8,

    /// BLE configuration when acting as a peripheral.
    ble_peripheral_config: ble::peripheral::Config,

    /// Global executor's task spawner.
    task_spawner: Spawner,
}

impl BluetoothServer {
    /// Initialize the `BluetoothServer`.
    pub fn new(device_name: &DeviceName, max_connections: u8, task_spawner: Spawner) -> Self {
        let bluetooth_device =
            BluetoothDevice::new(device_name, max_connections).run(&task_spawner);
        let ble_periph_config = ble::peripheral::Config::default();
        let num_connections = AtomicU8::new(0);

        Self {
            bluetooth_device,
            max_connections,
            num_connections,
            ble_peripheral_config: ble_periph_config,
            task_spawner,
        }
    }
}
