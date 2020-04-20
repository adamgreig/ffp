// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::{read_reg, write_reg, modify_reg, flash};

pub struct Flash {
    flash: flash::Instance,
}

const KEY1: u32 = 0x4567_0123;
const KEY2: u32 = 0xCDEF_89AB;
const OPT_RDP_ADDR:  u32 = 0x1FFF_F800;
const OPT_RDP_VALUE:  u8 = 0xAA;
const OPT_USER_ADDR: u32 = 0x1FFF_F802;
const OPT_USER_VALUE: u8 = 0x7F;

impl Flash {
    pub fn new(flash: flash::Instance) -> Self {
        Flash { flash }
    }

    /// Set up flash peripheral, with prefetch enabled
    /// and waitstate suitable for 48MHz operation.
    ///
    /// This also checks the user option byte is correctly set and
    /// will set it if necessary, triggering a device reset.
    pub fn setup(&self) {
        // Enable prefetch buffer and set suitable wait states for 48MHz operation
        write_reg!(flash, self.flash, ACR, PRFTBE: Enabled, LATENCY: WS1);

        // We need BOOT_SEL (bit 7) to be cleared to 0 so the system bootloader
        // doesn't read the BOOT0 pin and jump to user code after we
        // jump to it. The factory default is 1, so we will need to clear
        // this bit on first run. Other bits remain default 1, so program 0x7F.
        if self.read_option_byte() != OPT_USER_VALUE {
            self.set_option_byte(OPT_USER_VALUE);
        }
    }

    /// Read the current user option byte.
    fn read_option_byte(&self) -> u8 {
        ((read_reg!(flash, self.flash, OBR) & 0x0000_FF00) >> 8) as u8
    }

    /// Programs the user option byte to given value.
    /// This method will always trigger an immediate system reset.
    fn set_option_byte(&self, value: u8) -> ! {
        // Unlock flash and option byte.
        self.unlock_flash();
        self.unlock_opt();

        // Erase all option byte data.
        // Note this also clears readout protection,
        // which sets it to level 1.
        self.erase_opt();

        // Restore readout protection to disabled.
        self.program_opt_rdp(OPT_RDP_VALUE);

        // Program new user byte.
        self.program_opt_user(value);

        // Lock option byte and flash.
        self.lock_opt();
        self.lock_flash();

        // Trigger a reload of the option bytes, causing
        // an immediate system reset.
        self.reload_opt();
    }

    /// Unlock general flash operations.
    fn unlock_flash(&self) {
        write_reg!(flash, self.flash, KEYR, KEY1);
        write_reg!(flash, self.flash, KEYR, KEY2);
    }

    /// Unlock operations on option bytes.
    /// Flash access must be unlocked.
    fn unlock_opt(&self) {
        write_reg!(flash, self.flash, OPTKEYR, KEY1);
        write_reg!(flash, self.flash, OPTKEYR, KEY2);
    }

    /// Erase all option bytes, leaving them with all bits set.
    /// Flash and option access must be unlocked.
    fn erase_opt(&self) {
        // Select option byte erase operation
        modify_reg!(flash, self.flash, CR, OPTER: OptionByteErase);
        // Start erase operation
        modify_reg!(flash, self.flash, CR, STRT: Start);
        // Wait for erase completion
        while read_reg!(flash, self.flash, SR, BSY == Active) {}
        // Clear EOP flag
        modify_reg!(flash, self.flash, SR, EOP: 1);
        // Clear option byte erase setting
        modify_reg!(flash, self.flash, CR, OPTER: 0);
    }

    /// Program user option byte.
    /// Flash and option access must be unlocked.
    fn program_opt_user(&self, value: u8) {
        // Select option byte program operation
        modify_reg!(flash, self.flash, CR, OPTPG: OptionByteProgramming);
        // Program new value
        unsafe { core::ptr::write_volatile(OPT_USER_ADDR as *mut u16, value as u16) };
        // Wait for write completion
        while read_reg!(flash, self.flash, SR, BSY == Active) {}
        // Clear option byte program operation
        modify_reg!(flash, self.flash, CR, OPTPG: 0);
    }

    /// Program readout protection option byte.
    /// Flash and option access must be unlocked.
    fn program_opt_rdp(&self, value: u8) {
        // Select option byte program operation
        modify_reg!(flash, self.flash, CR, OPTPG: OptionByteProgramming);
        // Program new value
        unsafe { core::ptr::write_volatile(OPT_RDP_ADDR as *mut u16, value as u16) };
        // Wait for write completion
        while read_reg!(flash, self.flash, SR, BSY == Active) {}
        // Clear option byte program operation
        modify_reg!(flash, self.flash, CR, OPTPG: 0);
    }

    /// Lock access to option byte.
    /// Flash access must be unlocked.
    fn lock_opt(&self) {
        modify_reg!(flash, self.flash, CR, OPTWRE: Disabled);
    }

    /// Lock access to flash.
    fn lock_flash(&self) {
        modify_reg!(flash, self.flash, CR, LOCK: Locked);
    }

    /// Forces a reload of option bytes.
    /// This call triggers an immediate system reset.
    fn reload_opt(&self) -> ! {
        modify_reg!(flash, self.flash, CR, FORCE_OPTLOAD: Active);
        loop { continue; }
    }
}
