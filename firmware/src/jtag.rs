// Copyright 2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use crate::hal::gpio::{Pin, Pins};

struct JTAGPins<'a> {
    tms: &'a Pin<'a>,
    tck: &'a Pin<'a>,
    tdo: &'a Pin<'a>,
    tdi: &'a Pin<'a>,
}

pub struct JTAG<'a> {
    pins: JTAGPins<'a>,
}

impl<'a> JTAG<'a> {
    /// Create a new JTAG object from the provided Pins struct.
    pub fn new(pins: &'a Pins) -> Self {
        JTAG { pins: JTAGPins {
            tms: &pins.flash_si, tck: &pins.sck, tdo: &pins.cs, tdi: &pins.fpga_rst
        } }
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
    /// or conservatively as long as `data`, and must be initialised to all-0.
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

            // Run JTAG transfer, optionally capturing TDO.
            if capture != 0 {
                self.transfer(nbits, tms, tdi, Some(&mut rxbuf[rxidx..]));
                rxidx += nbytes;
            } else {
                self.transfer(nbits, tms, tdi, None);
            }
        }

        rxidx
    }

    /// Perform one JTAG transfer.
    ///
    /// Sets TMS to low if `tms` is 0, or high otherwise.
    /// Transmits `n` bits out of successive bytes of `tdi`, LSbit first.
    /// If `tdo` is `Some(&mut [u8])`, writes `n` bits into `tdo`, LSbit first.
    /// Otherwise if `tdo` is `None`, does not save received data.
    pub fn transfer(&self, n: usize, tms: u8, tdi: &[u8], tdo: Option<&mut [u8]>)
    {
        // Set TMS pin state.
        self.pins.tms.set_bool(tms != 0);

        // Perform either a read-write or a write-only transfer.
        match tdo {
            Some(tdo) => self.transfer_rw(n, tdi, tdo),
            None      => self.transfer_wo(n, tdi),
        }
    }

    /// Write-only JTAG transfer without capturing TDO.
    ///
    /// Writes `n` bits from successive bytes of `tdi`, LSbit first.
    fn transfer_wo(&self, n: usize, tdi: &[u8]) {
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

    /// Read-write JTAG transfer, with TDO capture.
    ///
    /// Writes `n` bits from successive bytes of `tdi`, LSbit first.
    /// Captures `n` bits from TDO and writes into successive bytes of `tdo`, LSbit first.
    fn transfer_rw(&self, n: usize, tdi: &[u8], tdo: &mut [u8]) {
        for (byte_idx, (tdi, tdo)) in tdi.iter().zip(tdo.iter_mut()).enumerate() {
            for bit_idx in 0..8 {
                // Stop after transmitting `n` bits.
                if byte_idx*8 + bit_idx == n {
                    return;
                }

                // Set TDI, read TDO, and toggle TCK.
                self.pins.tdi.set_bool(tdi & (1 << bit_idx) != 0);
                *tdo |= (self.pins.tdo.get_state() as u8) << bit_idx;
                self.pins.tck.set_high();
                self.pins.tck.set_low();
            }
        }
    }

    /// Compute required number of bytes to store a number of bits.
    fn bytes_for_bits(bits: usize) -> usize {
        (bits + 7) / 8
    }
}
