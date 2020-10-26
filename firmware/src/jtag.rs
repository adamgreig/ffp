// Copyright 2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::{read_reg, write_reg, gpio};
use crate::hal::gpio::{Pin, Pins};

struct JTAGPins<'a> {
    tms: &'a Pin<'a>,
    tck: &'a Pin<'a>,
    tdo: &'a Pin<'a>,
    tdi: &'a Pin<'a>,
}

pub struct JTAG<'a> {
    pins: JTAGPins<'a>,
    pins_same_port: bool,
}

impl<'a> JTAG<'a> {
    /// Create a new JTAG object from the provided Pins struct.
    pub fn new(pins: &'a Pins) -> Self {
        // If JTCK and JTDI and JTDO are on the same port, we can use a faster transfer method.
        let pins_same_port =
            (pins.sck.instance() as *const _ == pins.cs.instance() as *const _) &&
            (pins.sck.instance() as *const _ == pins.fpga_rst.instance() as *const _);

        JTAG { pins: JTAGPins {
            tms: &pins.flash_si, tck: &pins.sck, tdo: &pins.cs, tdi: &pins.fpga_rst
        }, pins_same_port }
    }

    /// Handle a sequence request. The request data follows the CMSIS-DAP
    /// DAP_JTAG_Sequence command:
    /// * First byte contains the number of sequences, then
    /// * First byte of each sequence contains:
    ///     * Bits 5..0: Number of clock cycles, where 0 means 64 cycles
    ///     * Bit 6: TMS value
    ///     * Bit 7: TDO capture enable
    /// * Subsequent bytes of each sequence contain TDI data, one bit per
    ///   clock cycle, with the final byte padded. Data is transmitted from
    ///   successive bytes, least significant bit first.
    ///
    /// Captured TDO data is written least significant bit first to successive
    /// bytes of `rxbuf`, which must be long enough for the requested capture,
    /// or conservatively as long as `data`.
    /// The final byte of TDO data for each sequence is padded, in other words,
    /// as many TDO bytes will be returned as there were TDI bytes in sequences
    /// with capture enabled.
    ///
    /// Returns the number of bytes of rxbuf which were written to.
    pub fn sequences(&self, data: &[u8], rxbuf: &mut [u8]) -> usize
    {
        // Read request header containing number of sequences.
        if data.len() == 0 { return 0 };
        let nseqs = data[0];
        let mut data = &data[1..];
        let mut rxidx = 0;

        // Process each sequence.
        for _ in 0..nseqs {
            // Read header byte for this sequence.
            if data.len() == 0 { break };
            let header = data[0];
            data = &data[1..];
            let capture = header & 0b1000_0000;
            let tms     = header & 0b0100_0000;
            let nbits   = header & 0b0011_1111;
            let nbits = if nbits == 0 { 64 } else { nbits as usize };
            let nbytes = Self::bytes_for_bits(nbits);
            if data.len() < nbytes { break };

            // Split data into TDI data for this sequence and data for remaining sequences.
            let tdi = &data[..nbytes];
            data = &data[nbytes..];

            // Set TMS for this transfer.
            self.pins.tms.set_bool(tms != 0);

            // Run one transfer, either read-write or write-only.
            if capture != 0 {
                self.transfer_rw(nbits, tdi, &mut rxbuf[rxidx..]);
                rxidx += nbytes;
            } else {
                self.transfer_wo(nbits, tdi);
            }
        }

        rxidx
    }

    /// Write-only JTAG transfer without capturing TDO.
    ///
    /// Writes `n` bits from successive bytes of `tdi`, LSbit first.
    fn transfer_wo(&self, n: usize, tdi: &[u8]) {
        if self.pins_same_port {
            return self.transfer_wo_fast(n, tdi);
        }

        for (byte_idx, byte) in tdi.iter().enumerate() {
            for bit_idx in 0..8 {
                // Stop after transmitting `n` bits.
                if byte_idx*8 + bit_idx == n {
                    return;
                }

                // Set TDI and toggle TCK.
                self.pins.tdi.set_bool(byte & (1 << bit_idx) != 0);
                self.pins.tck.set_high();
                self.pins.tck.set_low();
            }
        }
    }

    /// Write-only JTAG transfer without capturing TDO.
    ///
    /// This faster version requires that JTCK and JTDI are on the same GPIO port.
    fn transfer_wo_fast(&self, n: usize, tdi: &[u8]) {
        // Store all the relevant pins and ports for faster access
        let port = self.pins.tdi.instance();
        let tdi_pin = 1 << self.pins.tdi.pin_n();
        let tck_pin = 1 << self.pins.tck.pin_n();

        for (byte_idx, byte) in tdi.iter().enumerate() {
            for bit_idx in 0..8 {
                // Stop after transmitting `n` bits.
                if byte_idx*8 + bit_idx == n {
                    return;
                }

                // Set JTDI pin
                if byte & (1 << bit_idx) == 0 {
                    write_reg!(gpio, port, BRR, tdi_pin);
                } else {
                    write_reg!(gpio, port, BSRR, tdi_pin);
                }

                // Toggle JTCK pin
                write_reg!(gpio, port, BSRR, tck_pin);
                write_reg!(gpio, port, BRR, tck_pin);
            }
        }
    }

    /// Read-write JTAG transfer, with TDO capture.
    ///
    /// Writes `n` bits from successive bytes of `tdi`, LSbit first.
    /// Captures `n` bits from TDO and writes into successive bytes of `tdo`, LSbit first.
    fn transfer_rw(&self, n: usize, tdi: &[u8], tdo: &mut [u8]) {
        if self.pins_same_port {
            return self.transfer_rw_fast(n, tdi, tdo);
        }

        for (byte_idx, (tdi, tdo)) in tdi.iter().zip(tdo.iter_mut()).enumerate() {
            *tdo = 0;
            for bit_idx in 0..8 {
                // Stop after transmitting `n` bits.
                if byte_idx*8 + bit_idx == n {
                    return;
                }

                // Set TDI, read TDO, and toggle TCK.
                self.pins.tdi.set_bool(tdi & (1 << bit_idx) != 0);
                if self.pins.tdo.is_high() {
                    *tdo |= 1 << bit_idx;
                }
                self.pins.tck.set_high();
                self.pins.tck.set_low();
            }
        }
    }

    /// Read-write JTAG transfer, with TDO capture.
    ///
    /// This faster version requires JTCK, JTDI, and JTDO are all on the same GPIO port.
    fn transfer_rw_fast(&self, n: usize, tdi: &[u8], tdo: &mut [u8]) {
        // Store all the relevant pins and ports for faster access
        let port = self.pins.tdi.instance();
        let tdi_pin = 1 << self.pins.tdi.pin_n();
        let tdo_pin = 1 << self.pins.tdo.pin_n();
        let tck_pin = 1 << self.pins.tck.pin_n();

        for (byte_idx, (tdi, tdo)) in tdi.iter().zip(tdo.iter_mut()).enumerate() {
            *tdo = 0;
            for bit_idx in 0..8 {
                // Stop after transmitting `n` bits.
                if byte_idx*8 + bit_idx == n {
                    return;
                }

                // Set JTDI pin
                if tdi & (1 << bit_idx) == 0 {
                    write_reg!(gpio, port, BRR, tdi_pin);
                } else {
                    write_reg!(gpio, port, BSRR, tdi_pin);
                }

                // Read JTDO pin
                if read_reg!(gpio, port, IDR) & tdo_pin != 0 {
                    *tdo |= 1 << bit_idx;
                }

                // Toggle JTCK pin
                write_reg!(gpio, port, BSRR, tck_pin);
                write_reg!(gpio, port, BRR, tck_pin);
            }
        }
    }

    /// Compute required number of bytes to store a number of bits.
    fn bytes_for_bits(bits: usize) -> usize {
        (bits + 7) / 8
    }
}
