// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::spi;
use stm32ral::{write_reg, modify_reg, read_reg};

use super::dma::DMA;

pub struct SPI {
    spi: spi::Instance,
    rxbuf: [u8; 64],
}

impl SPI {
    pub fn new(spi: spi::Instance) -> Self {
        SPI { spi, rxbuf: [0u8; 64] }
    }

    pub fn setup(&self) {
        // 12MHz, SPI Mode 3 (CPOL=1 CPHA=1)
        write_reg!(spi, self.spi, CR1,
                   BIDIMODE: Unidirectional, CRCEN: Disabled, RXONLY: FullDuplex,
                   SSM: Enabled, SSI: SlaveNotSelected, LSBFIRST: MSBFirst,
                   BR: Div4, MSTR: Master, CPOL: IdleHigh, CPHA: SecondEdge,
                   SPE: Disabled);
        write_reg!(spi, self.spi, CR2,
                   FRXTH: Quarter, DS: EightBit, TXDMAEN: Enabled, RXDMAEN: Enabled);
    }

    pub fn setup_dap(&self) {
        write_reg!(spi, self.spi, CR1,
                   BIDIMODE: Unidirectional, CRCEN: Disabled, RXONLY: FullDuplex,
                   SSM: Enabled, SSI: SlaveNotSelected, LSBFIRST: LSBFirst,
                   BR: Div2, MSTR: Master, CPOL: IdleHigh, CPHA: SecondEdge,
                   SPE: Enabled);
    }

    pub fn tx4(&self, data: u8) {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: FourBit);
        unsafe { core::ptr::write_volatile(&self.spi.DR as *const _ as *mut u8, data) };
        while read_reg!(spi, self.spi, SR, TXE != Empty) {}
    }

    pub fn tx8(&self, data: u8) {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: EightBit);
        unsafe { core::ptr::write_volatile(&self.spi.DR as *const _ as *mut u8, data) };
        while read_reg!(spi, self.spi, SR, TXE != Empty) {}
    }

    pub fn tx16(&self, data: u16) {
        write_reg!(spi, self.spi, CR2, FRXTH: Half, DS: SixteenBit);
        unsafe { core::ptr::write_volatile(&self.spi.DR as *const _ as *mut u16, data) };
        while read_reg!(spi, self.spi, SR, TXE != Empty) {}
    }

    pub fn wait_busy(&self) {
        while read_reg!(spi, self.spi, SR, BSY == Busy) {}
    }

    pub fn rx8(&self) -> u8 {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: EightBit);
        unsafe { core::ptr::write_volatile(&self.spi.DR as *const _ as *mut u8, 0x00) };
        while read_reg!(spi, self.spi, SR, RXNE == Empty) {}
        unsafe { core::ptr::read_volatile(&self.spi.DR as *const _ as *const u8) }
    }

    pub fn rx8_chain_first(&self) -> u8 {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: EightBit);
        unsafe { core::ptr::write_volatile(&self.spi.DR as *const _ as *mut u8, 0x00) };
        unsafe { core::ptr::write_volatile(&self.spi.DR as *const _ as *mut u8, 0x00) };
        while read_reg!(spi, self.spi, SR, RXNE == Empty) {}
        unsafe { core::ptr::read_volatile(&self.spi.DR as *const _ as *const u8) }
    }

    pub fn rx8_chain(&self) -> u8 {
        unsafe { core::ptr::write_volatile(&self.spi.DR as *const _ as *mut u8, 0x00) };
        while read_reg!(spi, self.spi, SR, RXNE == Empty) {}
        unsafe { core::ptr::read_volatile(&self.spi.DR as *const _ as *const u8) }
    }

    pub fn rx8_chain_last(&self) -> u8 {
        while read_reg!(spi, self.spi, SR, RXNE == Empty) {}
        unsafe { core::ptr::read_volatile(&self.spi.DR as *const _ as *const u8) }
    }

    pub fn rx6(&self) -> u8 {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: SixBit);
        unsafe { core::ptr::write_volatile(&self.spi.DR as *const _ as *mut u8, 0x00) };
        while read_reg!(spi, self.spi, SR, RXNE == Empty) {}
        unsafe { core::ptr::read_volatile(&self.spi.DR as *const _ as *const u8) }
    }

    pub fn rx5(&self) -> u8 {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: 4);
        unsafe { core::ptr::write_volatile(&self.spi.DR as *const _ as *mut u8, 0x00) };
        while read_reg!(spi, self.spi, SR, RXNE == Empty) {}
        unsafe { core::ptr::read_volatile(&self.spi.DR as *const _ as *const u8) }
    }

    pub fn rx4(&self) -> u8 {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: FourBit);
        unsafe { core::ptr::write_volatile(&self.spi.DR as *const _ as *mut u8, 0x00) };
        while read_reg!(spi, self.spi, SR, RXNE == Empty) {}
        unsafe { core::ptr::read_volatile(&self.spi.DR as *const _ as *const u8) }
    }

    pub fn rx16(&self) -> u16 {
        write_reg!(spi, self.spi, CR2, FRXTH: Half, DS: SixteenBit);
        unsafe { core::ptr::write_volatile(&self.spi.DR as *const _ as *mut u16, 0x00) };
        while read_reg!(spi, self.spi, SR, RXNE == Empty) {}
        unsafe { core::ptr::read_volatile(&self.spi.DR as *const _ as *const u16) }
    }

    pub fn drain(&self) {
        // FIFO is 32 bits so ideally we'd make two 16-bit reads, but that screws
        // up the SPI's FIFO pointers and wrecks subsequent reads on later operations.
        // It's still pretty fast to just do a write and 4 reads instead of a for loop etc.
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: EightBit);
        unsafe { core::ptr::read_volatile(&self.spi.DR as *const _ as *const u8); }
        unsafe { core::ptr::read_volatile(&self.spi.DR as *const _ as *const u8); }
        unsafe { core::ptr::read_volatile(&self.spi.DR as *const _ as *const u8); }
        unsafe { core::ptr::read_volatile(&self.spi.DR as *const _ as *const u8); }
    }

    pub fn stop(&self) {
        while read_reg!(spi, self.spi, SR, BSY == Busy) {}
        write_reg!(spi, self.spi, CR1, SPE: Disabled);
    }

    pub fn exchange(&mut self, dma: &DMA, data: &[u8]) -> &[u8] {
        // Set up DMA transfer (configures NDTR and MAR and enables streams)
        dma.spi1_enable(data, &mut self.rxbuf[..data.len()]);

        // Start SPI transfer
        modify_reg!(spi, self.spi, CR1, SPE: Enabled);

        // Busy wait for RX DMA completion (at most 43µs)
        while dma.spi1_busy() {}

        // Disable SPI and DMA
        dma.spi1_disable();
        modify_reg!(spi, self.spi, CR1, SPE: Disabled);

        // Return reference to newly received data
        &self.rxbuf[..data.len()]
    }
}
