use stm32ral::{nvic, interrupt};
use stm32ral::{write_reg};

pub struct NVIC {
    nvic: nvic::Instance,
}

impl NVIC {
    pub fn new(nvic: nvic::Instance) -> Self {
        NVIC { nvic }
    }

    pub unsafe fn steal() -> Self {
        NVIC { nvic: nvic::NVIC::steal() }
    }

    pub fn setup(&self) {
        // Enable USB and SPI1 interrupts
        write_reg!(nvic, self.nvic, ISER, 1<<(interrupt::SPI1 as u8));
        write_reg!(nvic, self.nvic, ISER, 1<<(interrupt::USB as u8));
    }
}
