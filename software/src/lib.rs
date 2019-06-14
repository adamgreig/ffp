use failure_derive::Fail;

mod programmer;
mod flash;
mod fpga;

pub use programmer::Programmer;
pub use flash::Flash;
pub use fpga::FPGA;

#[derive(Fail, Debug)]
pub enum FFPError {
    #[fail(display="USB error: {}", _0)]
    USBError(#[cause] libusb::Error),

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
}

impl From<libusb::Error> for FFPError {
    fn from(error: libusb::Error) -> Self {
        FFPError::USBError(error)
    }
}

pub type Result<T> = std::result::Result<T, failure::Error>;
