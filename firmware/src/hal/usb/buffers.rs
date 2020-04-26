// Copyright 2019-2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

#[allow(non_snake_case)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BTableRow {
    pub ADDR_TX: u16,
    pub COUNT_TX: u16,
    pub ADDR_RX: u16,
    pub COUNT_RX: u16,
}

#[repr(C)]
pub struct EPBuf {
    // USB SRAM memory must be written by half-words, so we represent
    // the buffers as u16s which we'll manage writing u8 into.
    pub tx: [u16; 32],
    pub rx: [u16; 32],
}

/// Start address of USB SRAM. Values written to BTABLE are relative to this address.
pub const USB_SRAM: u32 = 0x4000_6000;

/// Global buffer for EP0, stored in USB SRAM
#[link_section=".usbram"]
pub static mut EP0BUF: EPBuf = EPBuf::new();

/// Global buffer for EP1, stored in USB SRAM
#[link_section=".usbram"]
pub static mut EP1BUF: EPBuf = EPBuf::new();

/// Global buffer for EP2, stored in USB SRAM
#[link_section=".usbram"]
pub static mut EP2BUF: EPBuf = EPBuf::new();

/// Global buffer for EP3, stored in USB SRAM
#[link_section=".usbram"]
pub static mut EP3BUF: EPBuf = EPBuf::new();

/// Global buffer for EP4, stored in USB SRAM
#[link_section=".usbram"]
pub static mut EP4BUF: EPBuf = EPBuf::new();

/// Global buffer table descriptors, stored in USB SRAM
#[link_section=".usbram"]
pub static mut BTABLE: [BTableRow; 8] = [BTableRow::new(); 8];

impl EPBuf {
    /// Create a new empty EPBuf
    pub const fn new() -> Self {
        EPBuf {
            tx: [0u16; 32], rx: [0u16; 32]
        }
    }

    /// Copy `data` into the tx buffer
    pub fn write_tx(&mut self, data: &[u8]) {
        let n = data.len();
        assert!(n <= self.tx.len() * 2);

        // We have to convert the incoming bytes to u16 words and write those,
        // as the USB SRAM memory region does not support u8 or u32 writes.
        // The reference manual says it supports u8 writes, but reality disagrees.
        // The input data might not be u16 aligned, so we can't simply cast it either.
        for (idx, chunk) in data.chunks_exact(2).enumerate() {
            let w = (chunk[0] as u16) | ((chunk[1] as u16) << 8);

            // A regular write can get optimised into a memcpy which wouldn't obey
            // the u16 write semantics, so use a manual volatile copy loop.
            unsafe { core::ptr::write_volatile(&mut self.tx[idx], w) };
        }

        // Handle final byte of odd-sized transfers
        if n & 1 == 1 {
            self.tx[n/2] = data[data.len() - 1] as u16;
        }
    }

    /// Copy rx buffer into `data`
    pub fn read_rx(&self, btable: &BTableRow, data: &mut [u8]) -> usize {
        let rx_len = btable.rx_count();
        assert!(data.len() >= rx_len);
        // Copy received data into `data`
        for (idx, word) in (&self.rx)[..rx_len/2].iter().enumerate() {
            let [u1, u2] = word.to_le_bytes();
            data[idx*2  ] = u1;
            data[idx*2+1] = u2;
        }
        // Handle final byte of odd-sized transfers
        if rx_len & 1 == 1 {
            data[rx_len - 1] = self.rx[rx_len/2] as u8;
        }
        // Return size of received data
        rx_len as usize
    }
}

impl BTableRow {
    /// Create a new empty BTableRow
    pub const fn new() -> Self {
        BTableRow { ADDR_TX: 0, COUNT_TX: 0, ADDR_RX: 0, COUNT_RX: 0 }
    }

    /// Set the COUNT_TX field to `n`
    pub fn tx_count(&mut self, n: usize) {
        self.COUNT_TX = n as u16;
    }

    /// Get the current COUNT_RX value
    pub fn rx_count(&self) -> usize {
        (self.COUNT_RX & 0x3FF) as usize
    }

    /// Writes buffer location and size to this BTableRow
    pub fn write(&mut self, buf: &EPBuf) {
        self.ADDR_TX = (&buf.tx as *const _ as u32 - USB_SRAM) as u16;
        self.ADDR_RX = (&buf.rx as *const _ as u32 - USB_SRAM) as u16;
        self.COUNT_TX = 0;
        self.COUNT_RX = (1<<15) | ((64/32 - 1) << 10);
    }
}
