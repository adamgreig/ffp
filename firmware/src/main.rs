// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

#![no_std]
#![no_main]

extern crate panic_halt;
use cortex_m_rt::{entry, pre_init};
use cortex_m_semihosting::hprintln;

pub mod hal;
pub mod app;
pub mod swd;

#[pre_init]
unsafe fn pre_init() {
    // Check if we should jump to system bootloader.
    //
    // When we receive the BOOTLOAD command over USB,
    // we write a flag to a static and reset the chip,
    // and `bootload::check()` will jump to the system
    // memory bootloader if the flag is present.
    //
    // It must be called from pre_init as otherwise the
    // flag is overwritten when statics are initialised.
    hal::bootload::check();
}

#[derive(Copy, Clone, Debug)]
pub enum DemoError {
    SWDError(swd::Error),
    UnexpectedValue(u32),
    BadCtrlStat(u32),
}

pub type Result<T> = core::result::Result<T, DemoError>;

impl core::convert::From<swd::Error> for DemoError {
    fn from(value: swd::Error) -> Self {
        DemoError::SWDError(value)
    }
}

const AHBAP_APIDR: u32 = 0x24770011;
const CM4_DPIDR:   u32 = 0x2ba01477;
const DHCSR: u32 = 0xE000EDF0;
const APIDR: u8 = 0b11;
const CSW: u8 = 0b00;
const TAR: u8 = 0b01;
const DRW: u8 = 0b11;

pub fn swd_read_mem(swd: &swd::SWD, addr: u32) -> Result<u32> {
    swd.write_dp(swd::DPRegister::SELECT, 0x0000_0000)?;
    swd.write_ap(CSW, 0x2300_0052)?;
    swd.write_ap(TAR, addr)?;
    swd.read_ap(DRW)?;
    match swd.read_dp(swd::DPRegister::CTRLSTAT)? {
        0xF000_0040 => (),
        x => Err(DemoError::BadCtrlStat(x))?,
    }
    Ok(swd.read_dp(swd::DPRegister::RDBUFF)?)
}

pub fn swd_read_bulk(swd: &swd::SWD, start_addr: u32, buf: &mut [u32]) -> Result<()> {
    swd.write_dp(swd::DPRegister::SELECT, 0x0000_0000)?;
    swd.write_ap(CSW, 0x2300_0052)?;
    swd.write_ap(TAR, start_addr)?;
    swd.read_ap(DRW)?;
    match swd.read_dp(swd::DPRegister::CTRLSTAT)? {
        0xF000_0040 => (),
        x => Err(DemoError::BadCtrlStat(x))?,
    }
    for x in buf.iter_mut() {
        *x = swd.read_ap(DRW)?;
    }
    Ok(())
}

pub fn swd_write_mem(swd: &swd::SWD, addr: u32, value: u32) -> Result<()> {
    swd.write_dp(swd::DPRegister::SELECT, 0x0000_0000)?;
    swd.write_ap(CSW, 0x2300_0052)?;
    swd.write_ap(TAR, addr)?;
    swd.write_ap(DRW, value)?;
    Ok(())
}

pub fn swd_demo(swd: &swd::SWD) -> Result<[u32; 4]> {

    // Sends line reset and JTAG-to-SWD transition
    swd.start();

    // Must read DPIDR first. Check it's correct for this Cortex-M4F.
    match swd.read_dp(swd::DPRegister::DPIDR)? {
        CM4_DPIDR => (),
        x => Err(DemoError::UnexpectedValue(x))?,
    };

    // Send CDBGPWRUPREQ and CSYSPWRUPREQ
    swd.write_dp(swd::DPRegister::CTRLSTAT, 0x5000_0000)?;

    // Wait to see CSYSPWRUPACK and CDBGPRUPACK
    while swd.read_dp(swd::DPRegister::CTRLSTAT)? & 0xF000_0000 != 0xF000_0000 {}

    // Read of APIDR
    swd.write_dp(swd::DPRegister::SELECT, 0x0000_00F0)?;
    swd.read_ap(APIDR)?;
    match swd.read_dp(swd::DPRegister::RDBUFF)? {
        AHBAP_APIDR => (),
        x => Err(DemoError::UnexpectedValue(x))?,
    }

    // Write 0xA05F0003 to DHCSR: C_HALT and C_DEBUGEN
    swd_write_mem(&swd, DHCSR, 0xA05F0003)?;

    // Let's read flash memory!
    let mut mem = [0u32; 4];
    swd_read_bulk(&swd, 0x0800_0000, &mut mem)?;
    Ok(mem)
}

#[entry]
fn main() -> ! {
    let rcc = hal::rcc::RCC::new(stm32ral::rcc::RCC::take().unwrap(),
                                 stm32ral::crs::CRS::take().unwrap());
    //let nvic = hal::nvic::NVIC::new(stm32ral::nvic::NVIC::take().unwrap(),
                                    //stm32ral::scb::SCB::take().unwrap());
    let gpioa = hal::gpio::GPIO::new(stm32ral::gpio::GPIOA::take().unwrap());
    let gpiob = hal::gpio::GPIO::new(stm32ral::gpio::GPIOB::take().unwrap());
    let spi = hal::spi::SPI::new(stm32ral::spi::SPI1::take().unwrap());

    // Define pinout
    let flash_si = gpioa.pin(7);
    let flash_si_input_mode = flash_si.memoise_mode_input();
    let flash_si_alternate_mode = flash_si.memoise_mode_alternate();
    let pins = hal::gpio::Pins {
        led: gpioa.pin(2),
        cs: gpioa.pin(3),
        fpga_rst: gpioa.pin(4),
        sck: gpioa.pin(5),
        flash_so: gpioa.pin(6),
        flash_si,
        fpga_so: gpiob.pin(4),
        fpga_si: gpiob.pin(5),
        tpwr_det: gpiob.pin(6),
        tpwr_en: gpiob.pin(7),

        flash_si_input_mode,
        flash_si_alternate_mode,
    };
    let swd = swd::SWD::new(&spi, &pins);

    rcc.setup();
    pins.setup();
    pins.swd_mode();
    pins.swd_tx();
    spi.setup_swd();

    // Power up target and wait for ST-Link to stop asserting reset
    pins.tpwr_en.set_high();
    cortex_m::asm::delay(82_000_000);

    // Run demo
    let demo_result = swd_demo(&swd);

    // Send some 1s to fix the buggy Saleae SWD analyser
    swd.idle_high();

    // Finished, power down target
    pins.tpwr_en.set_low();

    // See what we got
    match demo_result {
        Ok([w0, w1, w2, w3]) => {
            hprintln!("Demo ran OK, results: {:08x} {:08x} {:08x} {:08x}", w0, w1, w2, w3).ok();
        },
        Err(e) => {
            hprintln!("Demo error: {:?}", e).ok();
        },
    }

    loop {
        cortex_m::asm::nop();
    }
}
