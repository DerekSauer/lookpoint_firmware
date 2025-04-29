// SPDX-FileCopyrightText: 2025 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only
//

use embassy_executor::Spawner;
use embassy_futures::join::join;
use trouble_host::prelude::*;

use super::battery_service::BatteryService;

#[gatt_server]
pub struct GattServer {
    battery_service: BatteryService,
}

/// Launch the BLE GATT server then begin advertising and servicing connections.
///
/// # Remarks
///
/// The MCU's manufacturer has already assigned the device a MAC address.
/// Available from the DEVICEADDR fields in the factory information
/// configuration registers (FICR).
pub async fn run<'a, C>(
    device_name: DeviceName<'a>,
    device_address: [u8; 6],
    task_spawner: Spawner,
    ble_controller: C,
) where
    C: Controller,
{
    let address = Address::random(device_address);
    defmt::info!("[ble] device address is {}", address);

    // Construct a complete host controller interface. The resource pool manages
    // memory needed by the L2CAP interface between the host and the controller.
    let mut host_resources: HostResources<DefaultPacketPool, { super::NUM_PERIPH_CONNECTIONS }, 2> =
        HostResources::new();
    let ble_stack =
        trouble_host::new(ble_controller, &mut host_resources).set_random_address(address);
    let mut host = ble_stack.build();
    defmt::info!("[ble] host initialized");

    // Strictly speaking, our device *is* a motion sensor, though this BLE
    // appearance is intended for devices that detection motion in the space
    // around them (wall or pole mounted motion detectors).
    let gatt_server = match GattServer::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name:       device_name.as_ref(),
        appearance: &appearance::sensor::MOTION_SENSOR,
    })) {
        Ok(server) => {
            defmt::info!("[ble] GATT server initialized");
            server
        }
        Err(error) => {
            panic!("[ble] failed to initialize GATT server: {}", error);
        }
    };

    let _ = join(ble_task(host.runner), async {
        loop {
            match super::advertise::advertise(
                device_name.as_ref(),
                &mut host.peripheral,
                &gatt_server,
            )
            .await
            {
                Ok(connection) => {
                    let tick_task = tick_task(&connection, &ble_stack).await;
                }
                Err(error) => {
                    panic!("[ble] unable to start advertising: {:?}", error);
                }
            }
        }
    })
    .await;
}

async fn tick_task<C: Controller, P: PacketPool>(
    connection: &GattConnection<'_, '_, P>,
    stack: &Stack<'_, C, P>,
) {
    let mut tick: u8 = 0;

    loop {
        match connection.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                break;
            }
            GattConnectionEvent::PhyUpdated { tx_phy, rx_phy } => {}
            GattConnectionEvent::ConnectionParamsUpdated {
                conn_interval,
                peripheral_latency,
                supervision_timeout,
            } => {}
            GattConnectionEvent::Gatt { event } => match event {
                Ok(event) => {
                    event.accept().unwrap();
                }
                Err(error) => {
                    defmt::warn!("[gatt] error processing event: {:?}", error)
                }
            },
        }
    }
}

/// Task that will run the BLE host's event loop.
async fn ble_task<C: Controller, P: PacketPool>(mut runner: Runner<'_, C, P>) {
    loop {
        if let Err(error) = runner.run().await {
            panic!(
                "[ble] error occured in the BLE event loop task: {:?}",
                error
            );
        }
    }
}

// Wrapper around a BLE device name ensuring the name conforms to the 22 byte
// size limit.
pub struct DeviceName<'a>(&'a str);

impl<'a> DeviceName<'a> {
    // Create a new BLE device name. If the name is longer than 22 bytes it will be
    // truncated to the nearest UTF-8 codepoint that fits.
    pub fn new(device_name: &'a str) -> Self {
        const MAX_NAME_LEN: usize = 22;
        let device_name = if device_name.len() > MAX_NAME_LEN {
            let closest_uft8 = device_name.floor_char_boundary(MAX_NAME_LEN);
            let trunc_name = &device_name[..closest_uft8];
            defmt::warn!(
                "[ble] device name `{}` is longer than 22 bytes, truncating to `{}`",
                device_name,
                trunc_name
            );
            trunc_name
        } else {
            &device_name
        };

        Self(device_name)
    }
}

impl<'a> From<&'a str> for DeviceName<'a> {
    fn from(value: &'a str) -> Self {
        DeviceName::new(value)
    }
}

impl<'a> AsRef<&'a str> for DeviceName<'a> {
    fn as_ref(&self) -> &&'a str {
        &self.0
    }
}
