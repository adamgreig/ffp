// Copyright 2019-2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use num_enum::TryFromPrimitive;
use crate::{hal, dap};

#[derive(Copy, Clone, TryFromPrimitive)]
#[repr(u16)]
pub enum PinState {
    Low = 0,
    High = 1,
}

#[derive(Copy, Clone, TryFromPrimitive)]
#[repr(u16)]
pub enum Mode {
    HighImpedance = 0,
    Flash = 1,
    FPGA = 2,
}

#[derive(Copy, Clone)]
pub enum Request {
    SetCS(PinState),
    SetFPGA(PinState),
    SetTPwr(PinState),
    SetLED(PinState),
    SetMode(Mode),
    GetTPwr,
    Bootload,
    Suspend,
    SPITransmit(([u8; 64], usize)),
    DAP1Command(([u8; 64], usize)),
    DAP2Command(([u8; 64], usize)),
}

pub struct App<'a> {
    flash: &'a hal::flash::Flash,
    rcc: &'a hal::rcc::RCC,
    nvic: &'a hal::nvic::NVIC,
    dma: &'a hal::dma::DMA,
    pins: &'a hal::gpio::Pins<'a>,
    spi: &'a hal::spi::SPI,
    usb: &'a mut hal::usb::USB,
    dap: &'a mut dap::DAP<'a>,
}

impl<'a> App<'a> {
    pub fn new(flash: &'a hal::flash::Flash, rcc: &'a hal::rcc::RCC,
               nvic: &'a hal::nvic::NVIC, dma: &'a hal::dma::DMA,
               pins: &'a hal::gpio::Pins<'a>, spi: &'a hal::spi::SPI,
               usb: &'a mut hal::usb::USB, dap: &'a mut dap::DAP<'a>) -> Self {
        App {
            flash, rcc, nvic, dma, pins, spi, usb, dap,
        }
    }

    pub fn setup(&mut self) {
        // Configure flash latency to 1 wait state with prefetch
        self.flash.setup();
        // Configure system clock to HSI48 and enable CRS and peripheral clocks
        self.rcc.setup();
        // Enable SEVONPEND
        self.nvic.setup();
        // Configure DMA for SPI1 transfers
        self.dma.setup();
        // Configure GPIOs
        self.pins.setup();
        // Configure USB peripheral and connect to host
        self.usb.setup();
    }

    pub fn poll(&mut self) {
        if self.nvic.usb_pending() {
            // Handle USB interrupts
            if let Some(req) = self.usb.interrupt() {
                self.process_request(req);
            }
            self.nvic.unpend_usb();
        } else {
            // Sleep until an interrupt occurs
            cortex_m::asm::wfe();
        }
    }

    fn process_request(&mut self, req: Request) {
        match req {
            Request::SetCS(state) => self.pins.cs.set_state(state),
            Request::SetFPGA(state) => self.pins.fpga_rst.set_state(state),
            Request::SetTPwr(state) => self.pins.tpwr_en.set_state(state),
            Request::SetLED(state) => self.pins.led.set_state(state),
            Request::SetMode(mode) => match mode {
                Mode::HighImpedance => {
                    self.pins.high_impedance_mode();
                    self.usb.spi_data_disable();
                    self.usb.dap_enable();
                    self.spi.disable();
                },
                Mode::Flash => {
                    self.pins.flash_mode();
                    self.usb.spi_data_enable();
                    self.usb.dap_disable();
                    self.spi.setup_spi();
                },
                Mode::FPGA => {
                    self.pins.fpga_mode();
                    self.usb.spi_data_enable();
                    self.usb.dap_disable();
                    self.spi.setup_spi();
                },
            },
            Request::SPITransmit((txdata, n)) => {
                let mut rxdata = [0u8; 64];
                self.spi.exchange(&self.dma, &txdata[..n], &mut rxdata);
                self.usb.spi_data_reply(&rxdata[..n]);
            },
            Request::DAP1Command((report, n)) => {
                let response = self.dap.process_command(&report[..n]);
                if let Some(data) = response {
                    self.usb.dap1_reply(data);
                }
            },
            Request::DAP2Command((report, n)) => {
                let response = self.dap.process_command(&report[..n]);
                if let Some(data) = response {
                    self.usb.dap2_reply(data);
                }
            },
            Request::GetTPwr => self.usb.tpwr_reply(self.pins.tpwr_det.get_state()),
            Request::Bootload => hal::bootload::bootload(),
            Request::Suspend => {
                self.pins.high_impedance_mode();
                self.pins.led.set_low();
                self.pins.tpwr_en.set_low();
            },
        };
    }
}
