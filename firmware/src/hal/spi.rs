use stm32ral::spi;
use stm32ral::{write_reg, modify_reg};

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
