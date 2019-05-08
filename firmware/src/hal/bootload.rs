use stm32ral::{read_reg, write_reg, syscfg, scb, rtc, pwr, rcc};

const FLAG_VALUE: u32 = 0xB00110AD;

/// Call this function at boot before enabling any clocks or peripherals.
///
/// If we reset due to requesting a bootload, this function will jump to
/// the system bootloader.
pub fn check() {
    unsafe {
        // If flag isn't set we just continue with the boot process
        if read_reg!(rtc, RTC, BKP0R) != FLAG_VALUE {
            return;
        }

        // Otherwise, clear the flag and jump to system bootloader

        // Enable PWR clock, disable backup domain protection,
        // clear the flag in BKP0R, re-enable backup domain protection,
        // and disable PWR clock.
        write_reg!(rcc, RCC, APB1ENR, PWREN: Enabled);
        write_reg!(pwr, PWR, CR, DBP: 1);
        write_reg!(rtc, RTC, BKP0R, 0);
        write_reg!(pwr, PWR, CR, DBP: 0);
        write_reg!(rcc, RCC, APB1ENR, PWREN: Disabled);

        // Remap system memory to 0x0000_0000
        write_reg!(syscfg, SYSCFG, CFGR1, MEM_MODE: SystemFlash);

        // Get new stack pointer and jump address
        let sp = core::ptr::read_volatile(0 as *const u32);
        let rv = core::ptr::read_volatile(4 as *const u32);
        let bootloader: extern "C" fn() = core::mem::transmute(rv);

        // Write new stack pointer to MSP and call into system memory
        cortex_m::register::msp::write(sp);
        bootloader();
    }
}

/// Call this function to trigger a reset into the system bootloader
pub fn bootload() -> ! {
    unsafe {
        // Enable writing to backup domain, then write magic word to BKP0R
        write_reg!(pwr, PWR, CR, DBP: 1);
        write_reg!(rtc, RTC, BKP0R, 0xB00110AD);
        write_reg!(pwr, PWR, CR, DBP: 0);

        // Request system reset
        write_reg!(scb, SCB, AIRCR, VECTKEYSTAT: 0x05FA, SYSRESETREQ: 1);
    }

    // Wait for reset
    loop {
        cortex_m::asm::nop();
    }
}
