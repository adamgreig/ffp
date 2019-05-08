#![no_std]
#![no_main]

extern crate panic_halt;
use cortex_m_rt::{entry, pre_init};
use stm32ral::interrupt;

pub mod hal;

#[pre_init]
unsafe fn preinit() {
    // Check if we should jump to system bootloader
    hal::bootload::check();
}

#[entry]
fn main() -> ! {
    // Configure clocks
    let rcc = hal::rcc::RCC::new(stm32ral::rcc::RCC::take().unwrap(),
                                 stm32ral::crs::CRS::take().unwrap());
    rcc.setup();

    // Configure interrupts
    let nvic = hal::nvic::NVIC::new(stm32ral::nvic::NVIC::take().unwrap());
    nvic.setup();

    // Configure IO
    let gpioa = hal::gpio::GPIO::new(stm32ral::gpio::GPIOA::take().unwrap());
    let gpiob = hal::gpio::GPIO::new(stm32ral::gpio::GPIOB::take().unwrap());

    let led = gpioa.pin(2);
    let cs = gpioa.pin(3);
    let fpga_rst = gpioa.pin(4);
    let sck = gpioa.pin(5);
    let flash_so = gpioa.pin(6);
    let flash_si = gpioa.pin(7);
    let fpga_so = gpiob.pin(4);
    let fpga_si = gpiob.pin(5);
    let tpwr_det = gpiob.pin(6);
    let tpwr_en = gpiob.pin(7);

    led.set_mode_output().set_otype_pushpull().set_ospeed_low().clear();
    cs.set().set_otype_opendrain().set_ospeed_high().set_mode_output();
    fpga_rst.set().set_otype_opendrain().set_ospeed_high().set_mode_output();
    sck.set_mode_alternate().set_af(0).set_otype_pushpull().set_ospeed_veryhigh();
    flash_so.set_mode_input().set_af(0).set_otype_pushpull().set_ospeed_veryhigh();
    flash_si.set_mode_input().set_af(0).set_otype_pushpull().set_ospeed_veryhigh();
    fpga_so.set_mode_input().set_af(0).set_otype_pushpull().set_ospeed_veryhigh();
    fpga_si.set_mode_input().set_af(0).set_otype_pushpull().set_ospeed_veryhigh();
    tpwr_det.set_mode_input();
    tpwr_en.clear().set_mode_output().set_otype_pushpull().set_ospeed_low();

    // Configure SPI
    let spi = hal::spi::SPI::new(stm32ral::spi::SPI1::take().unwrap());
    spi.setup();

    // Configure USB
    let usb = hal::usb::USB::new(stm32ral::usb::USB::take().unwrap());
    usb.setup();

    loop {
        cortex_m::asm::wfi();
    }
}

#[interrupt]
unsafe fn USB() {
    hal::usb::USB::steal().interrupt();
}

#[interrupt]
unsafe fn SPI1() {
    hal::spi::SPI::new(stm32ral::spi::SPI1::steal()).interrupt();
}
