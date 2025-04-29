// SPDX-FileCopyrightText: 2025 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only

use trouble_host::prelude::*;

#[gatt_service(uuid = service::BATTERY)]
pub struct BatteryService {
    #[characteristic(uuid = characteristic::BATTERY_LEVEL, read, notify)]
    level: u8,
}
