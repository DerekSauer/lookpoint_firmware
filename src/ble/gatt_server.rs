// SPDX-FileCopyrightText: 2025 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-or-later

use trouble_host::prelude::*;

use super::services::device_information::DeviceInformation;

#[gatt_server]
pub struct GattServer {
    pub device_information: DeviceInformation,
}

impl<'values> GattServer<'values> {
    /// Start the Gatt server.
    pub fn start(device_name: &'values str) -> Result<Self, &'static str> {
        let gap_config = GapConfig::Peripheral(PeripheralConfig {
            name:       device_name,
            appearance: &appearance::light_fixtures::LIGHT_CONTROLLER,
        });

        GattServer::new_with_config(gap_config)
    }

    /// Process GATT events during connection intervals.
    pub async fn gatt_server_task<'gatt_server>(
        &self,
        connection: &GattConnection<'values, 'gatt_server, DefaultPacketPool>,
    ) {
        loop {
            match connection.next().await {
                GattConnectionEvent::Disconnected { reason } => {
                    defmt::debug!("[gatt] disconnected, ATT code: {}", reason);
                    break;
                }
                GattConnectionEvent::Gatt { event } => {
                    match &event {
                        GattEvent::Read(read_event) => {
                            defmt::debug!("[gatt] read event for handle: {}", &read_event.handle());
                        }
                        GattEvent::Write(write_event) => {
                            defmt::debug!(
                                "[gatt] write event for handle: {}",
                                &write_event.handle()
                            );
                        }
                        GattEvent::Other(_other_event) => {}
                    };

                    match event.accept() {
                        Ok(reply) => reply.send().await,
                        Err(err) => defmt::warn!("[gatt] error sending response: {:?}", err),
                    }
                }
                _ => {}
            }
        }

        defmt::debug!(
            "[gatt] connection event finished for handle: {}",
            connection.raw().handle().raw()
        );
    }
}
