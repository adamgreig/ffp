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
    let pins = hal::gpio::Pins {
        led: gpioa.pin(2),
        cs: gpioa.pin(3),
        fpga_rst: gpioa.pin(4),
        sck: gpioa.pin(5),
        flash_so: gpioa.pin(6),
        flash_si: gpioa.pin(7),
        fpga_so: gpiob.pin(4),
        fpga_si: gpiob.pin(5),
        tpwr_det: gpiob.pin(6),
        tpwr_en: gpiob.pin(7),
    };

    rcc.setup();
    pins.setup();
    spi.setup_dap();
    pins.dap_mode();
    pins.dap_tx();
    cortex_m::asm::delay(2_000_000);
    pins.tpwr_en.set_high();
    cortex_m::asm::delay(5_000_000);

    let id1;
    let mut id2 = None;
    let mut id3 = None;
    let mut abort = None;

    let swd = swd::SWD::new(&spi, &pins);
    swd.start();
    id1 = Some(swd.read_dp(swd::DPRegister::DPIDR));
    if id1.unwrap().is_ok() {
        id2 = Some(swd.read_dp(swd::DPRegister::DPIDR));
        if id2.unwrap().is_ok() {
            abort = Some(swd.write_dp(swd::DPRegister::DPIDR, 1));
            if abort.unwrap().is_ok() {
                id3 = Some(swd.read_dp(swd::DPRegister::DPIDR));
            }
        }
    }

    // send some 1s to fix the buggy Saleae SWD analyser
    swd.idle_high();

    // Finish
    pins.tpwr_en.set_low();

    match id1 {
        Some(Ok(id)) => { hprintln!("Read ID1: {:08x}", id).ok(); },
        Some(Err(e)) => { hprintln!("Error reading ID1: {:?}", e).ok(); },
        None => { hprintln!("No ID1").ok(); },
    }

    match id2 {
        Some(Ok(id)) => { hprintln!("Read ID2: {:08x}", id).ok(); },
        Some(Err(e)) => { hprintln!("Error reading ID2: {:?}", e).ok(); },
        None => { hprintln!("No ID2").ok(); },
    }

    match abort {
        Some(Ok(_)) => { hprintln!("Wrote abort OK").ok(); },
        Some(Err(e)) => { hprintln!("Error writing abort: {:?}", e).ok(); },
        None => { hprintln!("No abort").ok(); },
    }

    match id3 {
        Some(Ok(id)) => { hprintln!("Read ID3: {:08x}", id).ok(); },
        Some(Err(e)) => { hprintln!("Error reading ID3: {:?}", e).ok(); },
        None => { hprintln!("No ID3").ok(); },
    }

    loop {
        cortex_m::asm::nop();
    }
}
