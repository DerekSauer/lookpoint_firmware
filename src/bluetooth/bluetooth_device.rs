// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only

use super::device_name::DeviceName;
use nrf_softdevice::{raw as SoftdeviceAPI, Softdevice};

/// Connection to the microcontroller's Bluetooth device.
pub struct BluetoothDevice<S: BluetoothDeviceState> {
    internal_state: S,
}

/// Methods common to all `BluetoothDevice` states.
impl<S: BluetoothDeviceState> BluetoothDevice<S> {}

/// Methods available to an unconstructed `BluetoothDevice`.
impl BluetoothDevice<()> {
    /// Initialize the Bluetooth device and configure the Bluetooth controller.
    ///
    /// # Remarks
    /// `device_local_name` should not be longer than 19 bytes in length.
    /// If longer than 19 bytes the name will be truncated and a warning emitted.
    pub fn new(device_name: &DeviceName, max_connections: u8) -> BluetoothDevice<Enabled> {
        // UNWRAP: A `DeviceName` cannot be longer than 19 bytes. Safe conversion to u16.
        let device_name_length: u16 = device_name.len().try_into().unwrap();

        // Note: Some configuration fields require a u8 or u16 but the enums
        // automatically generated from nRF's headers are expressed as u32. It
        // is safe to truncate these values to the necessary size.

        // Clock configuration.
        let clock_config = SoftdeviceAPI::nrf_clock_lf_cfg_t {
            // Our board has an external 32Mhz oscillator.
            source: SoftdeviceAPI::NRF_CLOCK_LF_SRC_XTAL as u8,

            // With about 50PPM accuracy from -20 to +40 degrees celcius.
            accuracy: SoftdeviceAPI::NRF_CLOCK_LF_ACCURACY_50_PPM as u8,

            // Interval between timer calibrations. Zero with an external oscillator.
            // Presumably the external oscillator calibrates itself.
            rc_ctiv: 0,

            // Interval between calibrations if the temperature has not changed. Zero with an
            // external oscillator.
            rc_temp_ctiv: 0,
        };

        // Configuration parameters for the Generic Access Profile (GAP) layer.
        let conn_gap_config = SoftdeviceAPI::ble_gap_conn_cfg_t {
            // The number of concurrent connections the Bluetooth device can create.
            conn_count: max_connections,

            // How long the radio can be used to service a connection.
            event_length: SoftdeviceAPI::BLE_GAP_EVENT_LENGTH_DEFAULT as u16,
        };

        // Configuration parameters for the device's name.
        let gap_device_name = SoftdeviceAPI::ble_gap_cfg_device_name_t {
            // Pointer where the name is stored. Wants a mutable pointer but we'll
            // disable writing a new name so this pointer will not be mutated later.
            p_value: device_name.as_ptr() as *mut u8,

            // Name length will not change during runtime as changing the name is disabled.
            current_len: device_name_length,
            max_len: device_name_length,

            // Disable changing the device name during runtime. Security level (0, 0) is nRF
            // specific and not BLE standard.
            write_perm: SoftdeviceAPI::ble_gap_conn_sec_mode_t {
                _bitfield_1: SoftdeviceAPI::ble_gap_conn_sec_mode_t::new_bitfield_1(0, 0),
            },

            // The device name is stored in application memory.
            _bitfield_1: SoftdeviceAPI::ble_gap_cfg_device_name_t::new_bitfield_1(
                SoftdeviceAPI::BLE_GATTS_VLOC_USER as u8,
            ),
        };

        // Configure maximum concurrent connections in the `peripheral role`.
        // Our device will only behave as a peripheral so all connections are devoted to
        // this role.
        let gap_role_count = SoftdeviceAPI::ble_gap_cfg_role_count_t {
            adv_set_count: SoftdeviceAPI::BLE_GAP_ADV_SET_COUNT_DEFAULT as u8,

            // Maximum number of connections when acting as a peripheral.
            periph_role_count: max_connections,
        };

        // GATT configuration parameters.
        let conn_gatt = SoftdeviceAPI::ble_gatt_conn_cfg_t {
            // Maximum size of ATT packet the Bluetooth device can send or receive.
            att_mtu: SoftdeviceAPI::BLE_GATT_ATT_MTU_DEFAULT as u16,
        };

        let ble_config = nrf_softdevice::Config {
            clock: Some(clock_config),
            conn_gap: Some(conn_gap_config),
            gap_device_name: Some(gap_device_name),
            gap_role_count: Some(gap_role_count),
            conn_gatt: Some(conn_gatt),
            ..Default::default()
        };

        let softdevice = Softdevice::enable(&ble_config);

        defmt::info!("Bluetooth controller enabled.");

        BluetoothDevice {
            internal_state: Enabled { softdevice },
        }
    }
}

/// Attributes available to a `BluetoothDevice` in an `Enabled` state.
pub struct Enabled {
    /// Handle to the initialized nRF Softdevice Bluetooth controller.
    /// This reference is statically allocated in the Softdevice implementation.
    softdevice: &'static mut Softdevice,
}

/// Methods available to a `BluetoothDevice` in an `Enabled` state.
impl BluetoothDevice<Enabled> {
    /// Start the back ground task managing the Bluetooth controller's event
    /// loop.
    pub fn run(self, task_spawner: &embassy_executor::Spawner) -> BluetoothDevice<Running> {
        // Enable DC/DC mode so the Bluetooth controller can manage the DC/DC regulator.
        // When transmitting it will enable the regulator to efficiently supply
        // high voltage to the radio and switch back to LDO when not transmitting.
        // UNSAFE: Safe when the `BluetoothDevice` is in its `Enabled` state.
        unsafe {
            SoftdeviceAPI::sd_power_dcdc_mode_set(
                SoftdeviceAPI::NRF_POWER_DCDC_MODES_NRF_POWER_DCDC_ENABLE as u8,
            );
        }

        // We'll keep the mutable reference to the Bluetooth controller for our own uses
        // and send an immutable reference to the task running the controller's event
        // loop.
        // UNSAFE: Safe when the `BluetoothDevice` is in its `Enabled` state.
        let immutable_sd = unsafe { Softdevice::steal() };

        match task_spawner.spawn(softdevice_task(immutable_sd)) {
            Ok(_) => {
                defmt::info!("Bluetooth controller event loop started.");
            }
            Err(err) => match err {
                embassy_executor::SpawnError::Busy => {
                    panic!("Only one instance of the Bluetooth protocol stack task may be run.")
                }
            },
        }

        BluetoothDevice {
            internal_state: Running {
                softdevice: self.internal_state.softdevice,
            },
        }
    }
}

/// Attributes available to a `BluetoothDevice` in a `Running` state.
pub struct Running {
    /// Handle to the initialized nRF Softdevice Bluetooth controller.
    /// This reference is statically allocated in the Softdevice implementation.
    softdevice: &'static mut Softdevice,
}

/// Methods available to a `BluetoothDevice` in a `Running` state.
impl BluetoothDevice<Running> {}

/// Background task handling the Softdevice's event loop.
/// Only one `softdevice_task` may be running at the same time.
#[embassy_executor::task(pool_size = 1)]
async fn softdevice_task(softdevice: &'static Softdevice) {
    softdevice.run().await;
}

pub trait BluetoothDeviceState {}
impl BluetoothDeviceState for () {}
impl BluetoothDeviceState for Enabled {}
impl BluetoothDeviceState for Running {}
