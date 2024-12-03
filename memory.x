/*
 * SPDX-FileCopyrightText: 2024 Derek Sauer
 *
 * SPDX-License-Identifier: GPL-3.0-only
 */

/* nRF52840 with Softdevice S113 7.3.0 */
/* Softdevice consumes the first 112KB of flash and 64KB of RAM */
/* RAM use is a starting point until we determine real RAM need after configuring the device. */
MEMORY
{
     FLASH : ORIGIN = 0x0000000 + 112K, LENGTH = 1024K - 112K
     RAM : ORIGIN = 0x20000000 + 64K, LENGTH = 256K - 64K
}
