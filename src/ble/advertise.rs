// SPDX-FileCopyrightText: 2025 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only

use trouble_host::prelude::*;

use super::gatt_server::GattServer;

/// BLE advertising task. Runs until a connection is established and returns the
/// connection handle.
pub async fn advertise<'a, 'b, C: Controller>(
    device_name: &'a str,
    peripheral_role: &mut Peripheral<'a, C, DefaultPacketPool>,
    gatt_server: &'b GattServer<'_>,
) -> Result<GattConnection<'a, 'b, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut advertise_data = [0; 31];

    AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::CompleteLocalName(device_name.as_bytes()),
        ],
        &mut advertise_data[..],
    )?;

    let advertiser = peripheral_role
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data:  &advertise_data,
                scan_data: &[],
            },
        )
        .await?;

    defmt::info!("[ble] advertising started");

    let connection = advertiser
        .accept()
        .await
        .unwrap()
        .with_attribute_server(gatt_server)
        .unwrap();

    defmt::info!("[ble] connection established");

    Ok(connection)
}
