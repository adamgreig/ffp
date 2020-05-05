// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::dma1 as dma;
use stm32ral::{read_reg, write_reg, modify_reg};

use core::ops::{Deref, DerefMut};
use stable_deref_trait::StableDeref;
use as_slice::AsSlice;

const SPI_DR_OFFSET: u32 = 0x0C;
const UART_DR_OFFSET: u32 = 0x24;

pub unsafe trait Word {}
unsafe impl Word for u8 {}

pub unsafe trait DMAReadBuffer {
    type Word;
    fn dma_read_buffer(&self) -> (*const Self::Word, usize);
}

pub unsafe trait DMAWriteBuffer {
    type Word;
    fn dma_write_buffer(&mut self) -> (*mut Self::Word, usize);
}

unsafe impl <B, W> DMAReadBuffer for B
where
    B: Deref + StableDeref,
    B::Target: AsSlice<Element=W>,
    W: Word,
{
    type Word = W;

    fn dma_read_buffer(&self) -> (*const Self::Word, usize) {
        let slice = self.as_slice();
        (slice.as_ptr(), slice.len())
    }
}

pub unsafe trait DMAWriteTarget {
    type Word;
    fn as_dma_write_buffer(&mut self) -> (*mut Self::Word, usize);
}

unsafe impl <W: Word> DMAWriteTarget for [W] {
    type Word = W;
    fn as_dma_write_buffer(&mut self) -> (*mut Self::Word, usize) {
        (self.as_mut_ptr(), self.len())
    }
}

unsafe impl <B, T> DMAWriteBuffer for B
where
    B: DerefMut<Target = T> + StableDeref,
    T: DMAWriteTarget + ?Sized,
{
    type Word = T::Word;

    fn dma_write_buffer(&mut self) -> (*mut Self::Word, usize) {
        self.as_dma_write_buffer()
    }
}

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
                                       DIR: FromPeripheral, EN: Disabled);
        write_reg!(dma, self.dma, PAR2, stm32ral::spi::SPI1 as u32 + SPI_DR_OFFSET);

        // Set up channel 3 for SPI1_TX
        write_reg!(dma, self.dma, CR3, PL: High, MSIZE: Bits8, PSIZE: Bits8,
                                       MINC: Enabled, PINC: Disabled, CIRC: Disabled,
                                       DIR: FromMemory, EN: Disabled);
        write_reg!(dma, self.dma, PAR3, stm32ral::spi::SPI1 as u32 + SPI_DR_OFFSET);

        // Set up channel 5 for UART2_RX
        write_reg!(dma, self.dma, CR5, PL: High, MSIZE: Bits8, PSIZE: Bits8,
                                       MINC: Enabled, PINC: Disabled, CIRC: Enabled,
                                       DIR: FromPeripheral, EN: Disabled);
        write_reg!(dma, self.dma, PAR5, stm32ral::usart::USART2 as u32 + UART_DR_OFFSET);
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

    pub fn spi1_enable2<R, W>(&self, tx: R, mut rx: W)
    where
        R: DMAReadBuffer,
        W: DMAWriteBuffer,
    {
        let (src_ptr, src_len) = tx.dma_read_buffer();
        let (dst_ptr, dst_len) = rx.dma_write_buffer();
        write_reg!(dma, self.dma, IFCR, CGIF2: Clear, CGIF3: Clear);
        write_reg!(dma, self.dma, NDTR2, dst_len as u32);
        write_reg!(dma, self.dma, NDTR3, src_len as u32);
        write_reg!(dma, self.dma, MAR2, dst_ptr as u32);
        write_reg!(dma, self.dma, MAR3, src_ptr as u32);
        modify_reg!(dma, self.dma, CR2, EN: Enabled);
        modify_reg!(dma, self.dma, CR3, EN: Enabled);
    }

    /// Check if SPI1 transaction is still ongoing
    pub fn spi1_busy(&self) -> bool {
        read_reg!(dma, self.dma, ISR, TCIF2 == NotComplete)
    }

    /// Stop SPI1 DMA
    pub fn spi1_disable(&self) {
        modify_reg!(dma, self.dma, CR2, EN: Disabled);
        modify_reg!(dma, self.dma, CR3, EN: Disabled);
    }

    /// Start USART2 reception into provided buffer
    pub fn usart2_start(&self, rx: &mut [u8]) {
        write_reg!(dma, self.dma, IFCR, CGIF5: Clear);
        write_reg!(dma, self.dma, NDTR5, rx.len() as u32);
        write_reg!(dma, self.dma, MAR5, rx.as_mut_ptr() as u32);
        modify_reg!(dma, self.dma, CR5, EN: Enabled);
    }

    /// Return how many bytes are left to transfer for USART2
    pub fn usart2_ndtr(&self) -> usize {
        read_reg!(dma, self.dma, NDTR5) as usize
    }

    /// Stop USART2 DMA
    pub fn usart2_stop(&self) {
        modify_reg!(dma, self.dma, CR5, EN: Disabled);
    }
}
