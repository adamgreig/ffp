// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::{rcc, crs};
use stm32ral::{read_reg, modify_reg};

pub struct RCC {
    rcc: rcc::Instance,
    crs: crs::Instance,
}

impl RCC {
    pub fn new(rcc: rcc::Instance, crs: crs::Instance) -> Self {
        RCC { rcc, crs }
    }

    /// Set up the device, enabling all required clocks
    pub fn setup(&self) {
        // Turn on HSI48
        modify_reg!(rcc, self.rcc, CR2, HSI48ON: On);
        // Wait for HSI48 to be ready
        while read_reg!(rcc, self.rcc, CR2, HSI48RDY == NotReady) {}
        // Swap system clock to HSI48
        modify_reg!(rcc, self.rcc, CFGR, SW: HSI48);
        // Wait for system clock to be HSI48
        while read_reg!(rcc, self.rcc, CFGR, SWS != HSI48) {}

        // Enable peripheral clocks
        modify_reg!(rcc, self.rcc, AHBENR, IOPAEN: Enabled, IOPBEN: Enabled, DMAEN: Enabled);
        modify_reg!(rcc, self.rcc, APB1ENR, CRSEN: Enabled, USBEN: Enabled, USART2EN: Enabled);
        modify_reg!(rcc, self.rcc, APB2ENR, SPI1EN: Enabled);

        // Enable CRS (default CFGR values are appropriate for USB SOF sync)
        modify_reg!(crs, self.crs, CR, AUTOTRIMEN: 1, CEN: 1);
    }
}
