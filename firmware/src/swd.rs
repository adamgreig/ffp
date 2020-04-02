use crate::hal::{spi::SPI, gpio::Pins};

#[derive(Copy, Clone, Debug)]
pub enum Error {
    BadParity,
    AckWait,
    AckFault,
    AckProtocol,
    AckUnknown(u8),
    AckWaitTimeout,
    Other(&'static str),
}

pub type Result<T> = core::result::Result<T, Error>;

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum DPRegister {
    DPIDR       = 0,
    CTRLSTAT    = 1,
    SELECT      = 2,
    RDBUFF      = 3,
}

pub struct SWD<'a> {
    spi: &'a SPI,
    pins: &'a Pins<'a>,

    wait_retries: usize,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum APnDP {
    DP = 0,
    AP = 1,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum RnW {
    W = 0,
    R = 1,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum ACK {
    OK          = 0b001,
    WAIT        = 0b010,
    FAULT       = 0b100,
    PROTOCOL    = 0b111,
}

impl ACK {
    pub fn check_ok(ack: u8) -> Result<()> {
        match ack {
            v if v == (ACK::OK as u8) => Ok(()),
            v if v == (ACK::WAIT as u8) => Err(Error::AckWait),
            v if v == (ACK::FAULT as u8) => Err(Error::AckFault),
            v if v == (ACK::PROTOCOL as u8) => Err(Error::AckProtocol),
            _ => Err(Error::AckUnknown(ack)),
        }
    }
}

impl<'a> SWD<'a> {
    pub fn new(spi: &'a SPI, pins: &'a Pins) -> Self {
        SWD { spi, pins, wait_retries: 8 }
    }

    fn line_reset(&self) {
        // TODO: Change to 7. Seems to screw up the Saleae analyser at low clock speed though.
        for _ in 0..8 {
            self.spi.tx8(0xFF);
        }
    }

    fn jtag_to_swd(&self) {
        self.spi.tx16(0xE79E);
    }

    pub fn idle_high(&self) {
        self.spi.tx4(0xF);
    }

    pub fn idle_low(&self) {
        self.spi.tx4(0x0);
    }

    pub fn start(&self) {
        self.pins.swd_tx();
        self.line_reset();
        self.jtag_to_swd();
        self.line_reset();
        self.idle_low();
        self.spi.wait_busy();
    }

    pub fn read_dp(&self, a: DPRegister) -> Result<u32> {
        self.read(APnDP::DP, a as u8, self.wait_retries)
    }

    pub fn write_dp(&self, a: DPRegister, data: u32) -> Result<()> {
        self.write(APnDP::DP, a as u8, data, self.wait_retries)
    }

    pub fn read_ap(&self, a: u8) -> Result<u32> {
        self.read(APnDP::AP, a, self.wait_retries)
    }

    pub fn write_ap(&self, a: u8, data: u32) -> Result<()> {
        self.write(APnDP::AP, a, data, self.wait_retries)
    }

    fn read(&self, apndp: APnDP, a: u8, wait_retries: usize) -> Result<u32> {
        let req = Self::make_request(apndp, RnW::R, a);
        self.spi.tx8(req);
        self.spi.wait_busy();
        self.pins.swd_rx();
        self.spi.drain();

        // 1 clock for turnaround and 3 for ACK
        let ack = self.spi.rx4() >> 1;
        match ACK::check_ok(ack as u8) {
            Ok(_) => (),
            Err(Error::AckWait) if wait_retries > 0 => {
                self.pins.swd_tx();
                return self.read(apndp, a, wait_retries - 1);
            }
            Err(e) => {
                self.pins.swd_tx();
                return Err(e);
            },
        }

        // Read 8x4=32 bits of data and 8x1=8 bits for parity+turnaround+trailing.
        // Doing a batch of 5 8-bit reads is the quickest option as we keep the FIFO hot.
        let (data, parity) = self.spi.swd_rdata_phase(self.pins);
        let parity = (parity & 1) as u32;

        // Back to driving SWDIO to ensure it doesn't float high
        self.pins.swd_tx();

        match parity == (data.count_ones() & 1) {
            true => return Ok(data),
            false => return Err(Error::BadParity),
        }
    }

    fn write(&self, apndp: APnDP, a: u8, data: u32, wait_retries: usize) -> Result<()> {
        let req = Self::make_request(apndp, RnW::W, a);
        let parity = data.count_ones() & 1;

        self.spi.tx8(req);
        self.spi.wait_busy();
        self.pins.swd_rx();
        self.spi.drain();

        // 1 clock for turnaround and 3 for ACK and 1 for turnaround
        let ack = (self.spi.rx5() >> 1) & 0b111;
        match ACK::check_ok(ack as u8) {
            Ok(_) => (),
            Err(Error::AckWait) if wait_retries > 0 => {
                self.pins.swd_tx();
                return self.write(apndp, a, data, wait_retries - 1);
            }
            Err(e) => {
                self.pins.swd_tx();
                return Err(e);
            },
        }

        self.pins.swd_tx();

        // Write 8x4=32 bits of data and 8x1=8 bits for parity+trailing idle.
        // This way we keep the FIFO full and eliminate delays between words,
        // even at the cost of more trailing bits. We can't change DS to 4 bits
        // until the FIFO is empty, and waiting for that costs more time overall.
        // Additionally, many debug ports require a couple of clock cycles after
        // the parity bit of a write transaction to make the write effective.
        self.spi.swd_wdata_phase(data, parity as u8);
        self.spi.wait_busy();

        Ok(())
    }

    fn make_request(apndp: APnDP, rnw: RnW, a: u8) -> u8 {
        let req = (1 << 0) | ((apndp as u8) << 1) | ((rnw as u8) << 2) | (a << 3) | (1 << 7);
        let parity = (req.count_ones() & 1) as u8;
        req | (parity << 5)
    }
}
