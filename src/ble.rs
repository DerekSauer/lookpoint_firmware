// SPDX-FileCopyrightText: 2025 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only

mod battery_service;

pub mod advertise;
pub mod controller;
pub mod gatt_server;

/// Number of connections supported by the BLE peripheral role.
const NUM_PERIPH_CONNECTIONS: usize = 1;
