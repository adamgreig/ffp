use stm32ral::spi;
use stm32ral::{modify_reg};

pub struct SPI {
    spi: spi::Instance,
}

impl SPI {
    pub fn new(spi: spi::Instance) -> Self {
        SPI { spi }
    }

    pub fn setup(&self) {
        // 12MHz, SPI Mode 3 (CPOL=1 CPHA=1)
        modify_reg!(spi, self.spi, CR1,
                    BR: Div4, MSTR: Master, CPOL: IdleHigh, CPHA: SecondEdge, SPE: Enabled);
    }

    pub fn interrupt(&self) {
    }

    pub fn transmit(&self, _data: [u8; 64]) {
    }
}
