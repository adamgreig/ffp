// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::gpio;
use stm32ral::{read_reg, write_reg, modify_reg};
use crate::app::PinState;

pub struct GPIO {
    p: gpio::Instance,
}

pub struct Pin<'a> {
    n: u8,
    port: &'a GPIO,
}

pub struct Pins<'a> {
    pub led: Pin<'a>,
    pub cs: Pin<'a>,
    pub fpga_rst: Pin<'a>,
    pub sck: Pin<'a>,
    pub flash_so: Pin<'a>,
    pub flash_si: Pin<'a>,
    pub fpga_so: Pin<'a>,
    pub fpga_si: Pin<'a>,
    pub tpwr_det: Pin<'a>,
    pub tpwr_en: Pin<'a>,

    pub flash_si_input_mode: MemoisedMode,
    pub flash_si_alternate_mode: MemoisedMode,
    pub sck_output_mode: MemoisedMode,
    pub sck_alternate_mode: MemoisedMode,
}

/// Stores a pre-computed mask and value for quickly changing pin mode
#[derive(Copy, Clone)]
pub struct MemoisedMode {
    mask: u32,
    value: u32,
}

impl<'a> GPIO {
    pub fn new(p: gpio::Instance) -> Self {
        GPIO { p }
    }

    pub fn pin(&'a self, n: u8) -> Pin<'a> {
        assert!(n < 16);
        Pin { n, port: self }
    }

    pub fn set_high(&'a self, n: u8) -> &Self {
        write_reg!(gpio, self.p, BSRR, 1 << n);
        self
    }

    pub fn set_low(&'a self, n: u8) -> &Self {
        write_reg!(gpio, self.p, BRR, 1 << n);
        self
    }

    pub fn toggle(&'a self, n: u8) -> &Self {
        let pin = (read_reg!(gpio, self.p, IDR) >> n) & 1;
        if pin == 1 {
            self.set_low(n)
        } else {
            self.set_high(n)
        }
    }

    pub fn set_mode(&'a self, n: u8, mode: u32) -> &Self {
        let offset = n * 2;
        let mask = 0b11 << offset;
        let val = (mode << offset) & mask;
        modify_reg!(gpio, self.p, MODER, |r| (r & !mask) | val);
        self
    }

    pub const fn memoise_mode(n: u8, mode: u32) -> MemoisedMode {
        let n = n & 0xF;
        let offset = n * 2;
        let mask = 0b11 << offset;
        let value = (mode << offset) & mask;
        MemoisedMode { mask: !mask, value }
    }

    pub fn apply_memoised_mode(&'a self, mode: MemoisedMode) -> &Self {
        modify_reg!(gpio, self.p, MODER, |r| (r & mode.mask) | mode.value);
        self
    }

    pub fn set_mode_input(&'a self, n: u8) -> &Self {
        self.set_mode(n, gpio::MODER::MODER0::RW::Input)
    }

    pub const fn memoise_mode_input(n: u8) -> MemoisedMode {
        Self::memoise_mode(n, gpio::MODER::MODER0::RW::Input)
    }

    pub fn set_mode_output(&'a self, n: u8) -> &Self {
        self.set_mode(n, gpio::MODER::MODER0::RW::Output)
    }

    pub const fn memoise_mode_output(n: u8) -> MemoisedMode {
        Self::memoise_mode(n, gpio::MODER::MODER0::RW::Output)
    }

    pub fn set_mode_alternate(&'a self, n: u8) -> &Self {
        self.set_mode(n, gpio::MODER::MODER0::RW::Alternate)
    }

    pub const fn memoise_mode_alternate(n: u8) -> MemoisedMode {
        Self::memoise_mode(n, gpio::MODER::MODER0::RW::Alternate)
    }

    pub fn set_mode_analog(&'a self, n: u8) -> &Self {
        self.set_mode(n, gpio::MODER::MODER0::RW::Analog)
    }

    pub const fn memoise_mode_analog(n: u8) -> MemoisedMode {
        Self::memoise_mode(n, gpio::MODER::MODER0::RW::Analog)
    }

    pub fn set_otype(&'a self, n: u8, otype: u32) -> &Self {
        let offset = n;
        let mask = 0b1 << offset;
        let val = (otype << offset) & mask;
        modify_reg!(gpio, self.p, OTYPER, |r| (r & !mask) | val);
        self
    }

    pub fn set_otype_opendrain(&'a self, n: u8) -> &Self {
        self.set_otype(n, gpio::OTYPER::OT0::RW::OpenDrain)
    }

    pub fn set_otype_pushpull(&'a self, n: u8) -> &Self {
        self.set_otype(n, gpio::OTYPER::OT0::RW::PushPull)
    }

    pub fn set_ospeed(&'a self, n: u8, ospeed: u32) -> &Self {
        let offset = n * 2;
        let mask = 0b11 << offset;
        let val = (ospeed << offset) & mask;
        modify_reg!(gpio, self.p, OSPEEDR, |r| (r & !mask) | val);
        self
    }

    pub fn set_ospeed_low(&'a self, n: u8) -> &Self {
        self.set_ospeed(n, gpio::OSPEEDR::OSPEEDR0::RW::LowSpeed)
    }

    pub fn set_ospeed_medium(&'a self, n: u8) -> &Self {
        self.set_ospeed(n, gpio::OSPEEDR::OSPEEDR0::RW::MediumSpeed)
    }

    pub fn set_ospeed_high(&'a self, n: u8) -> &Self {
        self.set_ospeed(n, gpio::OSPEEDR::OSPEEDR0::RW::HighSpeed)
    }

    pub fn set_ospeed_veryhigh(&'a self, n: u8) -> &Self {
        self.set_ospeed(n, gpio::OSPEEDR::OSPEEDR0::RW::VeryHighSpeed)
    }

    pub fn set_af(&'a self, n: u8, af: u32) -> &Self {
        if n < 8 {
            let offset = n * 4;
            let mask = 0b1111 << offset;
            let val = (af << offset) & mask;
            modify_reg!(gpio, self.p, AFRL, |r| (r & !mask) | val);
        } else {
            let offset = (n - 8) * 4;
            let mask = 0b1111 << offset;
            let val = (af << offset) & mask;
            modify_reg!(gpio, self.p, AFRH, |r| (r & !mask) | val);
        }
        self
    }

    pub fn set_pull(&'a self, n: u8, pull: u32) -> &Self {
        let offset = n * 2;
        let mask = 0b11 << offset;
        let val = (pull << offset) & mask;
        modify_reg!(gpio, self.p, PUPDR, |r| (r & !mask) | val);
        self
    }

    pub fn set_pull_floating(&'a self, n: u8) -> &Self {
        self.set_pull(n, gpio::PUPDR::PUPDR0::RW::Floating)
    }

    pub fn set_pull_up(&'a self, n: u8) -> &Self {
        self.set_pull(n, gpio::PUPDR::PUPDR0::RW::PullUp)
    }

    pub fn set_pull_down(&'a self, n: u8) -> &Self {
        self.set_pull(n, gpio::PUPDR::PUPDR0::RW::PullDown)
    }

    pub fn get_idr(&'a self) -> u32 {
        read_reg!(gpio, self.p, IDR)
    }

    pub fn get_pin_idr(&'a self, n: u8) -> u32 {
        (self.get_idr() & (1 << n)) >> n
    }
}

impl<'a> Pin<'a> {
    pub fn set_high(&self) -> &Self {
        self.port.set_high(self.n);
        self
    }

    pub fn set_low(&self) -> &Self {
        self.port.set_low(self.n);
        self
    }

    pub fn set_state(&self, state: PinState) {
        match state {
            PinState::Low => self.set_low(),
            PinState::High => self.set_high(),
        };
    }

    pub fn set_bool(&self, state: bool) {
        if state {
            self.set_high();
        } else {
            self.set_low();
        }
    }

    pub fn get_state(&self) -> PinState {
        match self.port.get_pin_idr(self.n) {
            0 => PinState::Low,
            1 => PinState::High,
            _ => unreachable!(),
        }
    }

    pub fn is_high(&self) -> bool {
        match self.get_state() {
            PinState::High => true,
            PinState::Low => false,
        }
    }

    pub fn is_low(&self) -> bool {
        match self.get_state() {
            PinState::Low => true,
            PinState::High => false,
        }
    }

    pub fn toggle(&'a self) -> &Self {
        self.port.toggle(self.n);
        self
    }

    pub fn set_mode_input(&'a self) -> &Self {
        self.port.set_mode_input(self.n);
        self
    }

    pub fn set_mode_output(&'a self) -> &Self {
        self.port.set_mode_output(self.n);
        self
    }

    pub fn set_mode_alternate(&'a self) -> &Self {
        self.port.set_mode_alternate(self.n);
        self
    }

    pub fn set_mode_analog(&'a self) -> &Self {
        self.port.set_mode_analog(self.n);
        self
    }

    pub fn memoise_mode_input(&'a self) -> MemoisedMode {
        GPIO::memoise_mode_input(self.n)
    }

    pub fn memoise_mode_output(&'a self) -> MemoisedMode {
        GPIO::memoise_mode_output(self.n)
    }

    pub fn memoise_mode_alternate(&'a self) -> MemoisedMode {
        GPIO::memoise_mode_alternate(self.n)
    }

    pub fn memoise_mode_analog(&'a self) -> MemoisedMode {
        GPIO::memoise_mode_analog(self.n)
    }

    pub fn apply_memoised_mode(&'a self, mode: MemoisedMode) -> &Self {
        self.port.apply_memoised_mode(mode);
        self
    }

    pub fn set_otype_opendrain(&'a self) -> &Self {
        self.port.set_otype_opendrain(self.n);
        self
    }

    pub fn set_otype_pushpull(&'a self) -> &Self {
        self.port.set_otype_pushpull(self.n);
        self
    }

    pub fn set_ospeed_low(&'a self) -> &Self {
        self.port.set_ospeed_low(self.n);
        self
    }

    pub fn set_ospeed_medium(&'a self) -> &Self {
        self.port.set_ospeed_medium(self.n);
        self
    }

    pub fn set_ospeed_high(&'a self) -> &Self {
        self.port.set_ospeed_high(self.n);
        self
    }

    pub fn set_ospeed_veryhigh(&'a self) -> &Self {
        self.port.set_ospeed_veryhigh(self.n);
        self
    }

    pub fn set_af(&'a self, af: u32) -> &Self {
        self.port.set_af(self.n, af);
        self
    }

    pub fn set_pull_floating(&'a self) -> &Self {
        self.port.set_pull_floating(self.n);
        self
    }

    pub fn set_pull_up(&'a self) -> &Self {
        self.port.set_pull_up(self.n);
        self
    }

    pub fn set_pull_down(&'a self) -> &Self {
        self.port.set_pull_down(self.n);
        self
    }
}

impl<'a> Pins<'a> {
    /// Configure I/O pins
    pub fn setup(&self) {
        // Push-pull output to LED (active high).
        self.led
            .set_low()
            .set_otype_pushpull()
            .set_ospeed_low()
            .set_mode_output();

        // Open-drain output to FPGA and Flash CS (active low).
        // In SWD mode, AF1 USART2_RX.
        self.cs
            .set_high()
            .set_otype_opendrain()
            .set_ospeed_high()
            .set_pull_up()
            .set_af(1)
            .set_mode_input();

        // Open-drain output to FPGA reset line (active low).
        self.fpga_rst
            .set_high()
            .set_otype_opendrain()
            .set_ospeed_high()
            .set_mode_input();

        // Push-pull output to SPI SCK. Starts high-impedance.
        self.sck
            .set_af(0)
            .set_otype_pushpull()
            .set_ospeed_veryhigh()
            .set_mode_input();

        // Push-pull SPI MISO to Flash (FPGA MOSI). Starts high-impedance.
        // We also default to high so that when it is used as an open-drain
        // output for SWD mode (as the nRESET pin), it does not immediately
        // trigger a reset.
        self.flash_so
            .set_af(0)
            .set_otype_pushpull()
            .set_ospeed_veryhigh()
            .set_high()
            .set_mode_input();

        // Push-pull SPI MOSI to Flash (FPGA MISO). Starts high-impedance.
        self.flash_si
            .set_af(0)
            .set_otype_pushpull()
            .set_ospeed_veryhigh()
            .set_mode_input();

        // Push-pull SPI MISO to FPGA (Flash MOSI). Starts high-impedance.
        self.fpga_so
            .set_af(0)
            .set_otype_pushpull()
            .set_ospeed_veryhigh()
            .set_mode_input();

        // Push-pull SPI MOSI to FPGA (Flash MISO). Starts high-impedance.
        self.fpga_si
            .set_af(0)
            .set_otype_pushpull()
            .set_ospeed_veryhigh()
            .set_mode_input();

        // Input from target power rail.
        self.tpwr_det
            .set_mode_input();

        // Push-pull output drives target power switch (active high).
        self.tpwr_en
            .set_low()
            .set_mode_output()
            .set_otype_pushpull()
            .set_ospeed_low();
    }

    /// Place SPI pins into FPGA-programming mode
    pub fn fpga_mode(&self) {
        self.cs.set_mode_output();
        self.sck.set_mode_alternate();
        self.flash_so.set_mode_input();
        self.flash_si.set_mode_input();
        self.fpga_so.set_mode_alternate();
        self.fpga_si.set_mode_alternate();
        self.fpga_rst.set_otype_opendrain().set_mode_output();
    }

    /// Place SPI pins into flash-programming mode
    pub fn flash_mode(&self) {
        self.cs.set_mode_output();
        self.sck.set_mode_alternate();
        self.fpga_so.set_mode_input();
        self.fpga_si.set_mode_input();
        self.flash_so.set_otype_pushpull().set_mode_alternate();
        self.flash_si.set_mode_alternate();
        self.fpga_rst.set_otype_opendrain().set_mode_output();
    }

    /// Place SPI pins into high-impedance mode
    pub fn high_impedance_mode(&self) {
        self.cs.set_mode_input();
        self.sck.set_mode_input();
        self.flash_so.set_mode_input();
        self.flash_si.set_mode_input();
        self.fpga_so.set_mode_input().set_high();
        self.fpga_si.set_mode_input();
        self.fpga_rst.set_otype_opendrain().set_mode_output();
    }

    /// Place SPI pins into SWD mode:
    /// * sck pin is SPI clock
    /// * fpga_so pin is always MISO
    /// * flash_si pin is MOSI when we drive SWDIO or input when target drives it
    /// * flash_so is output open-drain for nRESET
    /// We don't change the actual state of flash_so in case it's already been
    /// used to drive nRESET low before attaching, but we reset it to high both
    /// at startup and after SWD detach.
    pub fn swd_mode(&self) {
        self.cs.set_mode_alternate();
        self.sck.set_mode_alternate().set_pull_up();
        self.flash_so.set_otype_opendrain().set_mode_output();
        self.flash_si.set_mode_alternate();
        self.fpga_so.set_mode_alternate();
        self.fpga_si.set_mode_input();
        self.fpga_rst.set_mode_input();
    }

    /// Place pins into JTAG mode:
    /// * sck pin is output push-pull for JTCK
    /// * flash_si is output push-pull for JTMS
    /// * cs is input for JTDO
    /// * fpga_rst is output push-pull for JTDI
    /// * flash_so pin is output open-drain for nRESET
    /// Other pins are left in high-impedance mode.
    /// We don't change the state of flash_so in case it's already been used to
    /// drive nRESET low before attaching to a target, but it is reset to high
    /// both at startup and after detaching.
    pub fn jtag_mode(&self) {
        self.sck.set_mode_output();
        self.flash_si.set_mode_output();
        self.cs.set_mode_input();
        self.fpga_rst.set_otype_pushpull().set_mode_output();
        self.flash_so.set_otype_opendrain().set_mode_output();
        self.fpga_si.set_mode_input();
        self.fpga_so.set_mode_input();
    }

    /// Disconnect MOSI from flash_si, target drives the bus
    pub fn swd_rx(&self) {
        self.flash_si.apply_memoised_mode(self.flash_si_input_mode);
    }

    /// Connect MOSI to flash_si, we drive the bus
    pub fn swd_tx(&self) {
        self.flash_si.apply_memoised_mode(self.flash_si_alternate_mode);
    }

    /// Swap clk pin to direct output mode for manual driving
    pub fn swd_clk_direct(&self) {
        self.sck.apply_memoised_mode(self.sck_output_mode);
    }

    /// Swap clk pin back to alternate mode for SPI use
    pub fn swd_clk_spi(&self) {
        self.sck.apply_memoised_mode(self.sck_alternate_mode);
    }
}
