// Copyright 2019-2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

#![allow(clippy::identity_op)]

use core::convert::{TryFrom, TryInto};
use num_enum::{TryFromPrimitive, IntoPrimitive};
use crate::{swd, hal::{gpio::Pins, spi::SPIClock, uart::UART}};

#[derive(Copy, Clone, TryFromPrimitive)]
#[allow(non_camel_case_types)]
#[repr(u8)]
enum Command {
    DAP_Info                = 0x00,
    DAP_HostStatus          = 0x01,
    DAP_Connect             = 0x02,
    DAP_Disconnect          = 0x03,
    DAP_WriteABORT          = 0x08,
    DAP_Delay               = 0x09,
    DAP_ResetTarget         = 0x0A,

    DAP_SWJ_Pins            = 0x10,
    DAP_SWJ_Clock           = 0x11,
    DAP_SWJ_Sequence        = 0x12,

    DAP_SWD_Configure       = 0x13,
    DAP_SWD_Sequence        = 0x1D,

    DAP_SWO_Transport       = 0x17,
    DAP_SWO_Mode            = 0x18,
    DAP_SWO_Baudrate        = 0x19,
    DAP_SWO_Control         = 0x1A,
    DAP_SWO_Status          = 0x1B,
    DAP_SWO_ExtendedStatus  = 0x1E,
    DAP_SWO_Data            = 0x1C,

    DAP_JTAG_Sequence       = 0x14,
    DAP_JTAG_Configure      = 0x15,
    DAP_JTAG_IDCODE         = 0x16,

    DAP_TransferConfigure   = 0x04,
    DAP_Transfer            = 0x05,
    DAP_TransferBlock       = 0x06,
    DAP_TransferAbort       = 0x07,

    DAP_ExecuteCommands     = 0x7F,
    DAP_QueueCommands       = 0x7E,

    Unimplemented           = 0xFF,
}

#[derive(Copy, Clone, IntoPrimitive)]
#[allow(non_camel_case_types)]
#[repr(u8)]
enum ResponseStatus {
    DAP_OK                  = 0x00,
    DAP_ERROR               = 0xFF,
}

#[derive(Copy, Clone, TryFromPrimitive)]
#[allow(non_camel_case_types)]
#[repr(u8)]
enum DAPInfoID {
    VendorID                = 0x01,
    ProductID               = 0x02,
    SerialNumber            = 0x03,
    FirmwareVersion         = 0x04,
    TargetVendor            = 0x05,
    TargetName              = 0x06,
    Capabilities            = 0xF0,
    TestDomainTimer         = 0xF1,
    SWOTraceBufferSize      = 0xFD,
    MaxPacketCount          = 0xFE,
    MaxPacketSize           = 0xFF,
}

#[derive(Copy, Clone, TryFromPrimitive)]
#[repr(u8)]
enum HostStatusType {
    Connect = 0,
    Running = 1,
}

#[derive(Copy, Clone, TryFromPrimitive)]
#[repr(u8)]
enum ConnectPort {
    Default = 0,
    SWD     = 1,
    JTAG    = 2,
}

#[repr(u8)]
enum ConnectPortResponse {
    Failed  = 0,
    SWD     = 1,

    #[allow(unused)]
    JTAG    = 2,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
enum SWOTransport {
    None        = 0,
    DAPCommand  = 1,
    USBEndpoint = 2,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
enum SWOMode {
    Off         = 0,
    UART        = 1,
    Manchester  = 2,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
enum SWOControl {
    Stop    = 0,
    Start   = 1,
}

struct Request<'a> {
    command: Command,
    data: &'a [u8],
}

impl <'a> Request<'a> {
    pub fn from_report(report: &'a [u8]) -> Option<Self> {
        if report.is_empty() {
            return None;
        }
        let command = Command::try_from(report[0]).ok()?;
        Some(Request { command, data: &report[1..] })
    }

    pub fn next_u8(&mut self) -> u8 {
        let value = self.data[0];
        self.data = &self.data[1..];
        value
    }

    pub fn next_u16(&mut self) -> u16 {
        let value = u16::from_le_bytes(self.data[0..2].try_into().unwrap());
        self.data = &self.data[2..];
        value
    }

    pub fn next_u32(&mut self) -> u32 {
        let value = u32::from_le_bytes(self.data[0..4].try_into().unwrap());
        self.data = &self.data[4..];
        value
    }

    pub fn rest(self) -> &'a [u8] {
        &self.data
    }
}

struct ResponseWriter<'a> {
    buf: &'a mut [u8],
    idx: usize,
}

impl <'a> ResponseWriter<'a> {
    pub fn new(command: Command, buf: &'a mut [u8]) -> Self {
        buf[0] = command as u8;
        ResponseWriter { buf, idx: 1 }
    }

    pub fn write_u8(&mut self, value: u8) {
        self.buf[self.idx] = value;
        self.idx += 1;
    }

    pub fn write_u16(&mut self, value: u16) {
        let value = value.to_le_bytes();
        self.buf[self.idx..self.idx+2].copy_from_slice(&value);
        self.idx += 2;
    }

    pub fn write_u32(&mut self, value: u32) {
        let value = value.to_le_bytes();
        self.buf[self.idx..self.idx+4].copy_from_slice(&value);
        self.idx += 4;
    }

    pub fn write_slice(&mut self, data: &[u8]) {
        self.buf[self.idx..self.idx + data.len()].copy_from_slice(&data);
        self.idx += data.len();
    }

    pub fn write_ok(&mut self) {
        self.write_u8(ResponseStatus::DAP_OK.into());
    }

    pub fn write_err(&mut self) {
        self.write_u8(ResponseStatus::DAP_ERROR.into());
    }

    pub fn write_u8_at(&mut self, idx: usize, value: u8) {
        self.buf[idx] = value;
    }

    pub fn write_u16_at(&mut self, idx: usize, value: u16) {
        let value = value.to_le_bytes();
        self.buf[idx..idx+2].copy_from_slice(&value);
    }

    pub fn mut_at(&mut self, idx: usize) -> &mut u8 {
        &mut self.buf[idx]
    }

    pub fn read_u8_at(&self, idx: usize) -> u8 {
        self.buf[idx]
    }

    pub fn finished(self) -> &'a [u8] {
        &self.buf[..self.idx]
    }
}

pub struct DAP<'a> {
    swd: swd::SWD<'a>,
    uart: &'a mut UART<'a>,
    pins: &'a Pins<'a>,
    rbuf: [u8; 64],
    configured: bool,
    swo_streaming: bool,
    match_retries: usize,
}

impl <'a> DAP<'a> {
    pub fn new(swd: swd::SWD<'a>, uart: &'a mut UART<'a>, pins: &'a Pins) -> Self
    {
        DAP {
            swd, uart, pins, rbuf: [0u8; 64],
            configured: false, swo_streaming: false,
            match_retries: 5,
        }
    }

    /// Process a new CMSIS-DAP command from `report`.
    ///
    /// Returns Some(response) if a response should be transmitted.
    pub fn process_command(&mut self, report: &[u8]) -> Option<&[u8]> {
        let req = Request::from_report(report)?;
        match req.command {
            Command::DAP_Info => self.process_info(req),
            Command::DAP_HostStatus => self.process_host_status(req),
            Command::DAP_Connect => self.process_connect(req),
            Command::DAP_Disconnect => self.process_disconnect(req),
            Command::DAP_WriteABORT => self.process_write_abort(req),
            Command::DAP_Delay => self.process_delay(req),
            Command::DAP_ResetTarget => self.process_reset_target(req),
            Command::DAP_SWJ_Pins => self.process_swj_pins(req),
            Command::DAP_SWJ_Clock => self.process_swj_clock(req),
            Command::DAP_SWJ_Sequence => self.process_swj_sequence(req),
            Command::DAP_SWD_Configure => self.process_swd_configure(req),
            Command::DAP_SWO_Transport => self.process_swo_transport(req),
            Command::DAP_SWO_Mode => self.process_swo_mode(req),
            Command::DAP_SWO_Baudrate => self.process_swo_baudrate(req),
            Command::DAP_SWO_Control => self.process_swo_control(req),
            Command::DAP_SWO_Status => self.process_swo_status(req),
            Command::DAP_SWO_ExtendedStatus => self.process_swo_extended_status(req),
            Command::DAP_SWO_Data => self.process_swo_data(req),
            Command::DAP_TransferConfigure => self.process_transfer_configure(req),
            Command::DAP_Transfer => self.process_transfer(req),
            Command::DAP_TransferBlock => self.process_transfer_block(req),
            Command::DAP_TransferAbort => self.process_transfer_abort(req),
            _ => Some(ResponseWriter::new(Command::Unimplemented, &mut self.rbuf)),
        }.map(|resp| resp.finished())
    }

    /// Returns true if SWO streaming is currently active.
    pub fn is_swo_streaming(&self) -> bool {
        self.uart.is_active() && self.swo_streaming
    }

    /// Polls the UART buffer for new SWO data, returning
    /// any data ready for streaming out the SWO EP.
    pub fn poll_swo(&mut self) -> Option<&[u8]> {
        self.uart.read(&mut self.rbuf)
    }

    fn process_info(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        match DAPInfoID::try_from(req.next_u8()) {
            // Return 0-length string for VendorID, ProductID, SerialNumber
            // to indicate they should be read from USB descriptor instead
            Ok(DAPInfoID::VendorID) => resp.write_u8(0),
            Ok(DAPInfoID::ProductID) => resp.write_u8(0),
            Ok(DAPInfoID::SerialNumber) => resp.write_u8(0),
            // Return git version as firmware version
            Ok(DAPInfoID::FirmwareVersion) => {
                resp.write_u8(crate::GIT_VERSION.len() as u8);
                resp.write_slice(crate::GIT_VERSION.as_bytes());
            },
            // Return 0-length string for TargetVendor and TargetName to indicate
            // unknown target device.
            Ok(DAPInfoID::TargetVendor) => resp.write_u8(0),
            Ok(DAPInfoID::TargetName) => resp.write_u8(0),
            Ok(DAPInfoID::Capabilities) => {
                resp.write_u8(1);
                // Bit 0: SWD supported
                // Bit 1: JTAG not supported
                // Bit 2: SWO UART supported
                // Bit 3: SWO Manchester not supported
                // Bit 4: Atomic commands not supported
                // Bit 5: Test Domain Timer not supported
                // Bit 6: SWO Streaming Trace supported
                resp.write_u8(0b0100_0101);
            },
            Ok(DAPInfoID::SWOTraceBufferSize) => {
                resp.write_u8(4);
                resp.write_u32(self.uart.buffer_len() as u32);
            },
            Ok(DAPInfoID::MaxPacketCount) => {
                resp.write_u8(1);
                // Maximum of one packet at a time
                resp.write_u8(1);
            },
            Ok(DAPInfoID::MaxPacketSize) => {
                resp.write_u8(2);
                // Maximum of 64 bytes per packet
                resp.write_u16(64);
            },
            _ => return None,
        }
        Some(resp)
    }

    fn process_host_status(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        let status_type = req.next_u8();
        let status_status = req.next_u8();
        // Use HostStatus to set our LED when host is connected to target
        if let Ok(HostStatusType::Connect) = HostStatusType::try_from(status_type) {
            match status_status {
                0 => { self.pins.led.set_low();  },
                1 => { self.pins.led.set_high(); },
                _ => (),
            }
        }
        resp.write_u8(0);
        Some(resp)
    }

    fn process_connect(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        let port = req.next_u8();
        match ConnectPort::try_from(port) {
            Ok(ConnectPort::Default) | Ok(ConnectPort::SWD) => {
                self.pins.swd_mode();
                self.swd.spi_enable();
                self.configured = true;
                resp.write_u8(ConnectPortResponse::SWD as u8);
            },
            _ => {
                resp.write_u8(ConnectPortResponse::Failed as u8);
            }
        }
        Some(resp)
    }

    fn process_disconnect(&mut self, req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        self.pins.high_impedance_mode();
        self.configured = false;
        self.swd.spi_disable();
        resp.write_ok();
        Some(resp)
    }

    fn process_write_abort(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        if !self.configured {
            resp.write_err();
            return Some(resp);
        }
        let _idx = req.next_u8();
        let word = req.next_u32();
        match self.swd.write_dp(0x00, word) {
            Ok(_) => resp.write_ok(),
            Err(_) => resp.write_err(),
        }
        Some(resp)
    }

    fn process_delay(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        let delay = req.next_u16() as u32;
        cortex_m::asm::delay(48 * delay);
        resp.write_ok();
        Some(resp)
    }

    fn process_reset_target(&mut self, req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        resp.write_ok();
        // "No device specific reset sequence is implemented"
        resp.write_u8(0);
        Some(resp)
    }

    fn process_swj_pins(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        let output = req.next_u8();
        let mask = req.next_u8();
        let wait = req.next_u32();

        // Our pin mapping:
        // SWDIO/TMS: FLASH_SI
        // SWCLK/TCK: SCK
        // SWO/TDO: FLASH_CS
        // TDI: FPGA_RST
        // nRESET: FLASH_SO
        //
        // SWJ_Pins mapping:
        // 0: SWCLK/TCK
        // 1: SWDIO/TMS
        // 2: TDI
        // 3: TDO
        // 5: nTRST
        // 7: nRESET
        //
        // We only support setting nRESET.

        const SWCLK_POS: u8 = 0;
        const SWDIO_POS: u8 = 1;
        const TDI_POS: u8 = 2;
        const TDO_POS: u8 = 3;
        const NTRST_POS: u8 = 5;
        const NRESET_POS: u8 = 7;
        const NRESET_MASK: u8 = 1<<NRESET_POS;

        // If reset bit is in mask, apply output bit to pin
        if (mask & NRESET_MASK) != 0 {
            if output & NRESET_MASK == 0 {
                // This command might be called to assert reset before
                // the DAP_Connect command which configures the I/O.
                self.pins.flash_so.set_otype_opendrain().set_low().set_mode_output();
            } else {
                // Set pin back to high-z when not being asserted.
                self.pins.flash_so.set_mode_input();
            }
        }

        // Delay required time in µs
        cortex_m::asm::delay(42 * wait);

        // Read and return pin state
        let state =
            ((self.pins.sck.get_state() as u8)      << SWCLK_POS)   |
            ((self.pins.flash_si.get_state() as u8) << SWDIO_POS)   |
            ((self.pins.fpga_rst.get_state() as u8) << TDI_POS)     |
            ((self.pins.cs.get_state() as u8)       << TDO_POS)     |
            (1                                      << NTRST_POS)   |
            ((self.pins.flash_so.get_state() as u8) << NRESET_POS);
        resp.write_u8(state);
        Some(resp)
    }

    fn process_swj_clock(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        let clock = req.next_u32();
        match SPIClock::from_max(clock) {
            Some(clk) => {
                self.swd.set_clock(clk);
                resp.write_ok();
            },
            None => {
                resp.write_err();
            },
        }
        Some(resp)
    }

    fn process_swj_sequence(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        let nbits: usize = match req.next_u8() {
            // CMSIS-DAP says 0 means 256 bits
            0 => 256,
            // We only support whole byte sequences at the moment,
            // but pyOCD sends 51 1s for line reset, with 7 bytes of 0xFF.
            // Remap 51 to 56 for this sneaky purpose.
            // I am sure it will not bite me later.
            51 => 56,
            // Other integers are normal.
            n => n as usize,
        };

        // We only support writing multiples of 8 bits
        if nbits % 8 != 0 {
            resp.write_err();
            return Some(resp);
        }

        let nbytes = nbits / 8;
        let seq = &req.rest()[..nbytes];
        self.swd.tx_sequence(seq);

        resp.write_ok();
        Some(resp)
    }

    fn process_swd_configure(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        let config = req.next_u8();
        let clk_period = config & 0b011;
        let always_data = (config & 0b100) != 0;
        if clk_period == 0 && !always_data {
            resp.write_ok();
        } else {
            resp.write_err();
        }
        Some(resp)
    }

    fn process_swo_transport(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        let transport = req.next_u8();
        match SWOTransport::try_from(transport) {
            Ok(SWOTransport::None) => {
                self.swo_streaming = false;
                resp.write_ok();
            },
            Ok(SWOTransport::DAPCommand) => {
                self.swo_streaming = false;
                resp.write_ok();
            },
            Ok(SWOTransport::USBEndpoint) => {
                self.swo_streaming = true;
                resp.write_ok();
            },
            _ => resp.write_err(),
        }
        Some(resp)
    }

    fn process_swo_mode(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        let mode = req.next_u8();
        match SWOMode::try_from(mode) {
            Ok(SWOMode::Off) => {
                resp.write_ok();
            },
            Ok(SWOMode::UART) => {
                resp.write_ok();
            },
            _ => resp.write_err(),
        }
        Some(resp)
    }

    fn process_swo_baudrate(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        let target = req.next_u32();
        let actual = self.uart.set_baud(target);
        resp.write_u32(actual);
        Some(resp)
    }

    fn process_swo_control(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        match SWOControl::try_from(req.next_u8()) {
            Ok(SWOControl::Stop) => {
                self.uart.stop();
                resp.write_ok();
            },
            Ok(SWOControl::Start) => {
                self.uart.start();
                resp.write_ok();
            },
            _ => resp.write_err(),
        }
        Some(resp)
    }

    fn process_swo_status(&mut self, req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        // Trace status:
        // Bit 0: trace capture active
        // Bit 6: trace stream error (always written as 0)
        // Bit 7: trace buffer overflow (always written as 0)
        resp.write_u8(self.uart.is_active() as u8);
        // Trace count: remaining bytes in buffer
        resp.write_u32(self.uart.bytes_available() as u32);
        Some(resp)
    }

    fn process_swo_extended_status(&mut self, req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        // Trace status:
        // Bit 0: trace capture active
        // Bit 6: trace stream error (always written as 0)
        // Bit 7: trace buffer overflow (always written as 0)
        resp.write_u8(self.uart.is_active() as u8);
        // Trace count: remaining bytes in buffer.
        resp.write_u32(self.uart.bytes_available() as u32);
        // Index: sequence number of next trace. Always written as 0.
        resp.write_u32(0);
        // TD_TimeStamp: test domain timer value for trace sequence
        resp.write_u32(0);
        Some(resp)
    }

    fn process_swo_data(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        // Limit maximum requested bytes to our maximum return size
        let n = usize::min(req.next_u16() as usize, 60);
        // Write status byte to response
        resp.write_u8(self.uart.is_active() as u8);
        // Read data from UART
        let mut buf = [0u8; 60];
        match self.uart.read(&mut buf[..n]) {
            None => {
                resp.write_u16(0);
            },
            Some(data) => {
                resp.write_u16(data.len() as u16);
                resp.write_slice(data);
            },
        }
        Some(resp)
    }

    fn process_transfer_configure(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);

        // We don't support variable idle cycles
        let _idle_cycles = req.next_u8();

        // Send number of wait retries through to SWD
        self.swd.set_wait_retries(req.next_u16() as usize);

        // Store number of match retries
        self.match_retries = req.next_u16() as usize;

        resp.write_ok();
        Some(resp)
    }

    fn process_transfer(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        let _idx = req.next_u8();
        let ntransfers = req.next_u8();
        let mut match_mask = 0xFFFF_FFFFu32;

        // Skip two bytes in resp to reserve space for final status,
        // which we update while processing.
        resp.write_u16(0);

        for transfer_idx in 0..ntransfers {
            // Store how many transfers we execute in the response
            resp.write_u8_at(1, transfer_idx + 1);

            // Parse the next transfer request
            let transfer_req = req.next_u8();
            let apndp   = (transfer_req & (1<<0)) != 0;
            let rnw     = (transfer_req & (1<<1)) != 0;
            let a       = (transfer_req & (3<<2)) >> 2;
            let vmatch  = (transfer_req & (1<<4)) != 0;
            let mmask   = (transfer_req & (1<<5)) != 0;
            let _ts     = (transfer_req & (1<<7)) != 0;

            if rnw {
                // Issue register read
                let mut read_value = if apndp {
                    // Reads from AP are posted, so we issue the
                    // read and subsequently read RDBUFF for the data.
                    // This requires an additional transfer so we'd
                    // ideally keep track of posted reads and just
                    // keep issuing new AP reads, but our reads are
                    // sufficiently fast that for now this is simpler.
                    let rdbuff = swd::DPRegister::RDBUFF.into();
                    if self.swd.read_ap(a).check(resp.mut_at(2)).is_none() {
                        break;
                    }
                    match self.swd.read_dp(rdbuff).check(resp.mut_at(2)) {
                        Some(v) => v,
                        None => break,
                    }
                } else {
                    // Reads from DP are not posted, so directly read the register.
                    match self.swd.read_dp(a).check(resp.mut_at(2)) {
                        Some(v) => v,
                        None => break,
                    }
                };

                // Handle value match requests by retrying if needed.
                // Since we're re-reading the same register the posting
                // is less important and we can just use the returned value.
                if vmatch {
                    let target_value = req.next_u32();
                    let mut match_tries = 0;
                    while (read_value & match_mask) != target_value {
                        match_tries += 1;
                        if match_tries > self.match_retries {
                            break;
                        }

                        read_value = match self.swd.read(apndp.into(), a).check(resp.mut_at(2)) {
                            Some(v) => v,
                            None => break,
                        }
                    }

                    // If we didn't read the correct value, set the value mismatch
                    // flag in the response and quit early.
                    if (read_value & match_mask) != target_value {
                        resp.write_u8_at(1, resp.read_u8_at(1) | (1<<4));
                        break;
                    }
                } else {
                    // Save read register value
                    resp.write_u32(read_value);
                }
            } else {
                // Write transfer processing

                // Writes with match_mask set just update the match mask
                if mmask {
                    match_mask = req.next_u32();
                    continue;
                }

                // Otherwise issue register write
                let write_value = req.next_u32();
                if self.swd.write(apndp.into(), a, write_value).check(resp.mut_at(2)).is_none() {
                    break;
                }
            }
        }

        Some(resp)
    }

    #[allow(clippy::collapsible_if)]
    fn process_transfer_block(&mut self, mut req: Request) -> Option<ResponseWriter> {
        let mut resp = ResponseWriter::new(req.command, &mut self.rbuf);
        let _idx = req.next_u8();
        let ntransfers = req.next_u16();
        let transfer_req = req.next_u8();
        let apndp = (transfer_req & (1<<0)) != 0;
        let rnw   = (transfer_req & (1<<1)) != 0;
        let a     = (transfer_req & (3<<2)) >> 2;

        // Skip three bytes in resp to reserve space for final status,
        // which we update while processing.
        resp.write_u16(0);
        resp.write_u8(0);

        // Keep track of how many transfers we executed,
        // so if there is an error the host knows where
        // it happened.
        let mut transfers = 0;

        // If reading an AP register, post first read early.
        if rnw && apndp {
            if self.swd.read_ap(a).check(resp.mut_at(3)).is_none() {
                // Quit early on error
                resp.write_u16_at(1, 1);
                return Some(resp);
            }
        }

        for transfer_idx in 0..ntransfers {
            transfers = transfer_idx;
            if rnw {
                // Handle repeated reads
                let read_value = if apndp {
                    // For AP reads, the first read was posted, so on the final
                    // read we need to read RDBUFF instead of the AP register.
                    if transfer_idx < ntransfers - 1 {
                        match self.swd.read_ap(a).check(resp.mut_at(3)) {
                            Some(v) => v,
                            None => break,
                        }
                    } else {
                        let rdbuff = swd::DPRegister::RDBUFF.into();
                        match self.swd.read_dp(rdbuff).check(resp.mut_at(3)) {
                            Some(v) => v,
                            None => break,
                        }
                    }
                } else {
                    // For DP reads, no special care required
                    match self.swd.read_dp(a).check(resp.mut_at(3)) {
                        Some(v) => v,
                        None => break,
                    }
                };

                // Save read register value to response
                resp.write_u32(read_value);
            } else {
                // Handle repeated register writes
                let write_value = req.next_u32();
                let result = self.swd.write(apndp.into(), a, write_value);
                if result.check(resp.mut_at(3)).is_none() {
                    break;
                }
            }
        }

        // Write number of transfers to response
        resp.write_u16_at(1, transfers + 1);

        // Return our response data
        Some(resp)
    }

    fn process_transfer_abort(&mut self, _req: Request) -> Option<ResponseWriter> {
        // We'll only ever receive an abort request when we're not already
        // processing anything else, since processing blocks checking for
        // new requests. Therefore there's nothing to do here.
        None
    }
}

trait CheckResult<T> {
    /// Check result of an SWD transfer, updating the response status byte.
    ///
    /// Returns Some(T) on successful transfer, None on error.
    fn check(self, resp: &mut u8) -> Option<T>;
}

impl<T> CheckResult<T> for swd::Result<T> {
    fn check(self, resp: &mut u8) -> Option<T> {
        match self {
            Ok(v) => {
                *resp = 1;
                Some(v)
            },
            Err(swd::Error::AckWait) => {
                *resp = 2;
                None
            },
            Err(swd::Error::AckFault) => {
                *resp = 4;
                None
            },
            Err(_) => {
                *resp = (1<<3) | 7;
                None
            }
        }
    }
}
