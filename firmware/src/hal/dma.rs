// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::dma1 as dma;
use stm32ral::{read_reg, write_reg, modify_reg};

const SPI_DR_OFFSET: u32 = 0x0C;

pub struct DMA {
    dma: dma::Instance,
}

impl DMA {
    pub fn new(dma: dma::Instance) -> Self {
        DMA { dma }
    }

    pub fn setup(&self) {
        // Set up channel 2 for SPI1_RX
        write_reg!(dma, self.dma, CR2, PL: High, MSIZE: Bits8, PSIZE: Bits8,
                                       MINC: Enabled, PINC: Disabled, CIRC: Disabled,
                                       DIR: FromPeripheral, TCIE: Disabled, EN: Disabled);
        write_reg!(dma, self.dma, PAR2, stm32ral::spi::SPI1 as u32 + SPI_DR_OFFSET);

        // Set up channel 3 for SPI1_TX
        write_reg!(dma, self.dma, CR3, PL: High, MSIZE: Bits8, PSIZE: Bits8,
                                       MINC: Enabled, PINC: Disabled, CIRC: Disabled,
                                       DIR: FromMemory, TCIE: Disabled, EN: Disabled);
        write_reg!(dma, self.dma, PAR3, stm32ral::spi::SPI1 as u32 + SPI_DR_OFFSET);
    }

    /// Sets up and enables a DMA transmit/receive for SPI1 (channels 2 and 3)
    pub fn spi1_enable(&self, tx: &[u8], rx: &mut [u8]) {
        write_reg!(dma, self.dma, IFCR, CGIF2: Clear, CGIF3: Clear);
        write_reg!(dma, self.dma, NDTR2, rx.len() as u32);
        write_reg!(dma, self.dma, NDTR3, tx.len() as u32);
        write_reg!(dma, self.dma, MAR2, rx.as_mut_ptr() as u32);
        write_reg!(dma, self.dma, MAR3, tx.as_ptr() as u32);
        modify_reg!(dma, self.dma, CR2, EN: Enabled);
        modify_reg!(dma, self.dma, CR3, EN: Enabled);
    }

    pub fn spi1_busy(&self) -> bool {
        read_reg!(dma, self.dma, ISR, TCIF2 == NotComplete)
    }

    pub fn spi1_disable(&self) {
        modify_reg!(dma, self.dma, CR2, EN: Disabled);
        modify_reg!(dma, self.dma, CR3, EN: Disabled);
    }
}
