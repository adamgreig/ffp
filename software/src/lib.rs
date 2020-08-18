use failure_derive::Fail;

mod programmer;
mod flash;
mod ice40;
mod jtag;
mod ecp5;

pub use programmer::Programmer;
pub use flash::{Flash, SPIFlash, FlashAccess};
pub use ice40::ICE40;
pub use jtag::JTAG;
pub use ecp5::ECP5;

#[derive(Fail, Debug)]
pub enum FFPError {
    #[fail(display="USB error: {}", _0)]
    USBError(#[cause] rusb::Error),

    #[fail(display="No FFP device found")]
    NoDeviceFound,

    #[fail(display="Multiple FFP devices found. Choose one with --index or --serial.")]
    MultipleDevicesFound,

    #[fail(display="Specified FFP device not found.")]
    DeviceNotFound,

    #[fail(display="Not enough data read back from device: expected {}, read {}", expected, read)]
    NotEnoughData { expected: usize, read: usize },

    #[fail(display="Flash readback verification failed")]
    ReadbackError,

    #[fail(display="An unknown error has occurred.")]
    UnknownError,

    #[fail(display="No ECP5 device found.")]
    ECP5NotFound,
}

impl From<rusb::Error> for FFPError {
    fn from(error: rusb::Error) -> Self {
        FFPError::USBError(error)
    }
}

pub type Result<T> = std::result::Result<T, failure::Error>;
