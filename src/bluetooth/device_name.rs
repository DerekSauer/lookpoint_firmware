// SPDX-FileCopyrightText: 2025 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only

/// Maximum permitted length of a BLE device name, in bytes.
const MAX_LOCAL_NAME_LENGTH: usize = 19;

/// A string slice appropriate for use as a BLE device name.
pub struct DeviceName<'a>(&'a str);

impl<'a> DeviceName<'a> {
    /// Create a new BLE device name.
    ///
    /// A device name must be 19 `bytes` or shorter in length. If a longer string slice is passed in,
    /// the returned value will be a slice of that string that fits within the limit. The string will
    /// be truncated at the nearest UTF-8 code point that fits within the limit.
    pub fn new(name: &'a str) -> Self {
        let device_name = if name.len() > MAX_LOCAL_NAME_LENGTH {
            let mut index = MAX_LOCAL_NAME_LENGTH;
            while !name.is_char_boundary(index) {
                index -= 1;
            }

            defmt::warn!(
                "Bluetooth device local name `{}` exceeds {} bytes, truncating to `{}`.",
                &name,
                MAX_LOCAL_NAME_LENGTH,
                &name[..index]
            );

            &name[..index]
        } else {
            name
        };

        DeviceName(device_name)
    }

    /// Returns the `DeviceName` length, in bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Converts a `DeviceName` into a raw pointer.
    /// The pointer points to the first byte of the `DeviceName`.
    pub fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }
}

impl<'a> From<&'a str> for DeviceName<'a> {
    fn from(value: &'a str) -> Self {
        Self::new(&value)
    }
}
