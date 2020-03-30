// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

#![no_std]
#![no_main]

extern crate panic_halt;
use cortex_m_rt::{entry, pre_init};
use cortex_m_semihosting::hprintln;

pub mod hal;
pub mod app;

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
    pins.dap_tx_mode();
    cortex_m::asm::delay(2_000_000);

    pins.tpwr_en.set_high();
    cortex_m::asm::delay(5_000_000);

    // 64 clocks with line high
    for _ in 0..4 {
        spi.tx16(0xFFFF);
    }

    // JTAG-to-SWD sequence
    spi.tx16(0xE79E);

    // 64 clocks with line high
    for _ in 0..4 {
        spi.tx16(0xFFFF);
    }

    // 16 clocks with line low
    spi.tx16(0x0000);

    // Read ID register
    spi.tx8(0xA5);

    pins.dap_tx_to_rx();
    spi.drain();
    let w1 = spi.rx16() as u32;
    let w2 = spi.rx16() as u32;
    let w3 = spi.rx8() as u32;

    spi.stop();

    pins.tpwr_en.set_low();

    //let ack: u32 = (w1 >> 1) & 0b111;
    //let data: u32 = (w1 >> 4) | (w2 << 4) | (w3 << 12) | (w4 << 20) | ((w5 & 0b1111) << 28);
    //let parity: u32 = w5 & 0b00010000;
    let ack: u32 = (w1 >> 1) & 0b111;
    let data: u32 = (w1 >> 4) | (w2 << 12) | ((w3 & 0b1111) << 28);
    let parity: u32 = w3 & 0b00010000;

    //hprintln!("{:02X} {:02X} {:02X} {:02X} {:02X}", w1, w2, w3, w4, w5).ok();
    hprintln!("{:04X} {:04X} {:02X}", w1, w2, w3).ok();
    hprintln!("ack={:03b} data={:08X} parity={}", ack, data, parity).ok();

    loop {
        cortex_m::asm::nop();
    }
}
