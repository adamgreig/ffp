// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

#![no_std]
#![no_main]

extern crate panic_halt;
use cortex_m_rt::{entry, pre_init};
use git_version::git_version;

const GIT_VERSION: &str = git_version!();

pub mod hal;
pub mod app;
pub mod swd;
pub mod dap;
pub mod jtag;

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
    let flash = hal::flash::Flash::new(stm32ral::flash::Flash::take().unwrap());
    let rcc = hal::rcc::RCC::new(stm32ral::rcc::RCC::take().unwrap(),
                                 stm32ral::crs::CRS::take().unwrap());
    let nvic = hal::nvic::NVIC::new(stm32ral::nvic::NVIC::take().unwrap(),
                                    stm32ral::scb::SCB::take().unwrap());
    let dma = hal::dma::DMA::new(stm32ral::dma1::DMA1::take().unwrap());
    let gpioa = hal::gpio::GPIO::new(stm32ral::gpio::GPIOA::take().unwrap());
    let gpiob = hal::gpio::GPIO::new(stm32ral::gpio::GPIOB::take().unwrap());
    let spi = hal::spi::SPI::new(stm32ral::spi::SPI1::take().unwrap());
    let mut uart = hal::uart::UART::new(stm32ral::usart::USART2::take().unwrap(), &dma);
    let mut usb = hal::usb::USB::new(stm32ral::usb::USB::take().unwrap());

    // Define pinout.
    // Some pins are defined early so we can memoise their modes for
    // faster mode switching at runtime.
    let sck = gpioa.pin(5);
    let flash_si = gpioa.pin(7);
    let flash_si_input_mode = flash_si.memoise_mode_input();
    let flash_si_alternate_mode = flash_si.memoise_mode_alternate();
    let sck_output_mode = sck.memoise_mode_output();
    let sck_alternate_mode = sck.memoise_mode_alternate();
    let pins = hal::gpio::Pins {
        led: gpioa.pin(2),
        cs: gpioa.pin(3),
        fpga_rst: gpioa.pin(4),
        sck,
        flash_so: gpioa.pin(6),
        flash_si,
        fpga_so: gpiob.pin(4),
        fpga_si: gpiob.pin(5),
        tpwr_det: gpiob.pin(6),
        tpwr_en: gpiob.pin(7),

        flash_si_input_mode,
        flash_si_alternate_mode,
        sck_output_mode,
        sck_alternate_mode,
    };

    let swd = swd::SWD::new(&spi, &pins);
    let jtag = jtag::JTAG::new(&pins);
    let mut dap = dap::DAP::new(swd, &jtag, &mut uart, &pins);

    // Create App instance with the HAL instances
    let mut app = app::App::new(
        &flash, &rcc, &nvic, &dma, &pins, &spi, &jtag, &mut usb, &mut dap);

    // Initialise application, including system peripherals
    app.setup();

    loop {
        // Process events
        app.poll();
    }
}
