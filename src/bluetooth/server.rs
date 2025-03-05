// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only

use super::{
    bluetooth_device::{BluetoothDevice, Running},
    device_name::DeviceName,
};

/// Bluetooth server managing the Bluetooth device, handling advertising, and
/// exposing services to connected clients.
pub struct BluetoothServer<'executor> {
    /// Connection to the initialized and running Bluetooth controller.
    bluetooth_device: BluetoothDevice<Running>,

    /// Maximum number of connections the Bluetooth server can manage.
    max_connections: u8,

    /// Reference to the Embassy executor's task spawner.
    task_spawner: &'executor embassy_executor::Spawner,
}

impl<'executor> BluetoothServer<'executor> {
    /// Start the Bluetooth Server.
    pub fn new(
        device_name: &DeviceName,
        max_connections: u8,
        task_spawner: &'executor embassy_executor::Spawner,
    ) -> Self {
        Self {
            bluetooth_device: BluetoothDevice::new(device_name, max_connections).run(task_spawner),
            max_connections,
            task_spawner,
        }
    }
}
