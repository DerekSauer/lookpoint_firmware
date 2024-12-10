/*
 * SPDX-FileCopyrightText: 2024 Derek Sauer
 *
 * SPDX-License-Identifier: GPL-3.0-only
 */

/* Note: The BLE Softdevice is a Bluetooth protocol stack provided by Nordic Semiconductor for nRF52 series microcontrollers. */
 
/* Memory address where flash storage begins */
FLASH_ORIGIN = 0x00000000;

/* Total flash available on the nRF52840 */
TOTAL_FLASH_SIZE = 1024K;

/* Memory offset where SRAM begins */
RAM_ORIGIN = 0x20000000;

/* Total RAM available on the nRF52840 */
TOTAL_RAM_SIZE = 256K;

/* Flash reserved for the BLE Softdevice S113 v7.3.0 */
SOFTDEVICE_FLASH_SIZE = 112K;

/* RAM reserved for the BLE Softdevice S113 v7.3.0 */
SOFTDEVICE_RAM_SIZE = 7712;

MEMORY
{
     /* Our code is written to flash after the BLE Softdevice */
     FLASH : ORIGIN = FLASH_ORIGIN + SOFTDEVICE_FLASH_SIZE, LENGTH = TOTAL_FLASH_SIZE - SOFTDEVICE_FLASH_SIZE

     /* The BLE Softdevice reserves the memory it requires at the start of RAM */
     RAM : ORIGIN = RAM_ORIGIN + SOFTDEVICE_RAM_SIZE, LENGTH = TOTAL_RAM_SIZE - SOFTDEVICE_RAM_SIZE
}
