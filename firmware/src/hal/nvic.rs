// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::{nvic, scb, Interrupt};
use stm32ral::{read_reg, write_reg};

pub struct NVIC {
    nvic: nvic::Instance,
    scb: scb::Instance,
}

impl NVIC {
    pub fn new(nvic: nvic::Instance, scb: scb::Instance) -> Self {
        NVIC { nvic, scb }
    }

    pub fn setup(&self) {
        // Set SEVONPEND to enable wake from WFE due to pending but disabled interrupt
        const SEVONPEND: u32 = 4;
        write_reg!(scb, self.scb, SCR, 1<<SEVONPEND);
    }

    pub fn is_pending(&self, interrupt: Interrupt) -> bool {
        read_reg!(nvic, self.nvic, ISPR) & (1<<(interrupt as u8)) != 0
    }

    pub fn usb_pending(&self) -> bool {
        self.is_pending(Interrupt::USB)
    }

    pub fn spi1_pending(&self) -> bool {
        self.is_pending(Interrupt::SPI1)
    }

    pub fn dma_ch_2_3_pending(&self) -> bool {
        self.is_pending(Interrupt::DMA1_CH2_3)
    }

    pub fn unpend(&self, interrupt: Interrupt) {
        write_reg!(nvic, self.nvic, ICPR, 1<<(interrupt as u8));
    }

    pub fn unpend_usb(&self) {
        self.unpend(Interrupt::USB);
    }

    pub fn unpend_spi1(&self) {
        self.unpend(Interrupt::SPI1);
    }

    pub fn unpend_dma_ch_2_3(&self) {
        self.unpend(Interrupt::DMA1_CH2_3);
    }
}
