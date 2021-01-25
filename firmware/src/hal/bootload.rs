// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

#![allow(clippy::zero_ptr, clippy::unreadable_literal)]

use stm32ral::{write_reg, syscfg, scb};

static mut FLAG: u32 = 0;
const FLAG_VALUE: u32 = 0xB00110AD;

/// Call this function at boot in pre_init, before statics are initialised.
///
/// If we reset due to requesting a bootload, this function will jump to
/// the system bootloader.
pub fn check() {
    unsafe {
        // If flag isn't set we just continue with the boot process
        if core::ptr::read_volatile(&FLAG) != FLAG_VALUE {
            return;
        }

        // Otherwise, clear the flag and jump to system bootloader
        core::ptr::write_volatile(&mut FLAG, 0);

        // Remap system memory to 0x0000_0000
        write_reg!(syscfg, SYSCFG, CFGR1, MEM_MODE: SystemFlash);

        // Jump using bootloader's vector table at address 0.
        cortex_m::asm::bootload(0 as *const u32);
    }
}

/// Call this function to trigger a reset into the system bootloader
pub fn bootload() -> ! {
    unsafe {
        // Write flag value to FLAG
        core::ptr::write_volatile(&mut FLAG, FLAG_VALUE);

        // Request system reset
        write_reg!(scb, SCB, AIRCR, VECTKEYSTAT: 0x05FA, SYSRESETREQ: 1);
    }

    // Wait for reset
    loop {
        cortex_m::asm::nop();
    }
}
