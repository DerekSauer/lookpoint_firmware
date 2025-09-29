// SPDX-FileCopyrightText: 2025 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-or-later

use trouble_host::prelude::*;

use super::services::device_information::DeviceInformation;

/// Begin advertising and wait for connections.
pub async fn advertise<'values, 'server, C: Controller>(
    device_name: &'values str,
    peripheral_role: &mut Peripheral<'values, C, DefaultPacketPool>,
    gatt_server: &'server super::gatt_server::GattServer<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut advertise_data = [0; 31];

    AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[DeviceInformation::BLE_UUID16.to_le_bytes()]),
            AdStructure::CompleteLocalName(device_name.as_bytes()),
        ],
        &mut advertise_data[..],
    )?;

    let advertiser = peripheral_role
        .advertise(
            &AdvertisementParameters::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data:  &advertise_data[..],
                scan_data: &[],
            },
        )
        .await?;

    let connection = advertiser
        .accept()
        .await?
        .with_attribute_server(gatt_server)?;

    Ok(connection)
}

/// BLE advertisement task.
/// Continually advertises until a connection is established. The connection is
/// then handed off to the GATT server for processing.
pub async fn advertise_task<'values, C: Controller>(
    device_name: &'values str,
    peripheral_role: &mut Peripheral<'values, C, DefaultPacketPool>,
    gatt_server: &super::gatt_server::GattServer<'values>,
) {
    loop {
        if let Ok(connection) = advertise(device_name, peripheral_role, gatt_server).await {
            gatt_server.gatt_server_task(&connection).await;
        }
    }
}
