use super::buffers::*;

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
pub struct SetupPID {
    pub bmRequestType: u8,
    pub bRequest: u8,
    pub wValue: u16,
    pub wIndex: u16,
    pub wLength: u16,
}

#[repr(u8)]
pub enum StandardRequest {
    ClearFeature = 1,
    GetConfiguration = 8,
    GetDescriptor = 6,
    GetInterface = 10,
    GetStatus = 0,
    SetAddress = 5,
    SetConfiguration = 9,
    SetDescriptor = 7,
    SetFeature = 3,
    SetInterface = 11,
    SynchFrame = 12,
}

#[repr(u8)]
pub enum VendorRequest {
    SetCS = 1,
    SetFPGA = 2,
    SetMode = 3,
    SetTPwr = 4,
    GetTPwr = 5,
    SetLED = 6,
    Bootload = 7,
}

#[repr(u8)]
pub enum DescriptorType {
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
}

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
pub struct DeviceDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bcdUSB: u16,
    pub bDeviceClass: u8,
    pub bDeviceSubClass: u8,
    pub bDeviceProtocol: u8,
    pub bMaxPacketSize0: u8,
    pub idVendor: u16,
    pub idProduct: u16,
    pub bcdDevice: u16,
    pub iManufacturer: u8,
    pub iProduct: u8,
    pub iSerialNumber: u8,
    pub bNumConfigurations: u8,
}

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
pub struct ConfigurationDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub wTotalLength: u16,
    pub bNumInterfaces: u8,
    pub bConfigurationValue: u8,
    pub iConfiguration: u8,
    pub bmAttributes: u8,
    pub bMaxPower: u8,
}

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
pub struct InterfaceDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bInterfaceNumber: u8,
    pub bAlternateSetting: u8,
    pub bNumEndpoints: u8,
    pub bInterfaceClass: u8,
    pub bInterfaceSubClass: u8,
    pub bInterfaceProtocol: u8,
    pub iInterface: u8,
}

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
pub struct EndpointDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bEndpointAddress: u8,
    pub bmAttributes: u8,
    pub wMaxPacketSize: u16,
    pub bInterval: u8,
}

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
pub struct StringDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bString: [u8; 32],
}

#[allow(unused)]
pub enum SetupDirection {
    HostToDevice = 0,
    DeviceToHost = 1,
}

#[derive(PartialEq)]
pub enum SetupType {
    Standard = 0,
    Class = 1,
    Vendor = 2,
    Reserved = 3,
}

#[allow(unused)]
pub enum SetupRecipient {
    Device = 0,
    Interface = 1,
    Endpoint = 2,
    Other = 3,
    Unknown,
}

impl SetupPID {
    pub fn from_buf(buf: &EPBuf) -> Self {
        let [req_type, req] = buf.rx[0].to_le_bytes();
        SetupPID {
            bmRequestType: req_type,
            bRequest: req,
            wValue: buf.rx[1],
            wIndex: buf.rx[2],
            wLength: buf.rx[3],
        }
    }

    #[allow(unused)]
    pub fn setup_direction(&self) -> SetupDirection {
        match (self.bmRequestType & (0b1 << 7)) >> 5 {
            0 => SetupDirection::HostToDevice,
            1 => SetupDirection::DeviceToHost,
            _ => unreachable!(),
        }
    }

    pub fn setup_type(&self) -> SetupType {
        match (self.bmRequestType & (0b11 << 5)) >> 5 {
            0 => SetupType::Standard,
            1 => SetupType::Class,
            2 => SetupType::Vendor,
            3 => SetupType::Reserved,
            _ => unreachable!(),
        }
    }

    #[allow(unused)]
    pub fn setup_recipient(&self) -> SetupRecipient {
        match self.bmRequestType & 0b11111 {
            0 => SetupRecipient::Device,
            1 => SetupRecipient::Interface,
            2 => SetupRecipient::Endpoint,
            3 => SetupRecipient::Other,
            _ => SetupRecipient::Unknown,
        }
    }
}

impl DescriptorType {
    /// Attempt to convert a u8 to a DescriptorType
    pub fn from_u8(x: u8) -> Option<Self> {
        match x {
            x if x == DescriptorType::Device as u8 =>
                Some(DescriptorType::Device),
            x if x == DescriptorType::Configuration as u8 =>
                Some(DescriptorType::Configuration),
            x if x == DescriptorType::String as u8 =>
                Some(DescriptorType::String),
            x if x == DescriptorType::Interface as u8 =>
                Some(DescriptorType::Interface),
            x if x == DescriptorType::Endpoint as u8 =>
                Some(DescriptorType::Endpoint),
            _ => None,
        }
    }
}

impl StandardRequest {
    /// Attempt to convert a u8 to a StandardRequest
    pub fn from_u8(x: u8) -> Option<Self> {
        match x {
            x if x == StandardRequest::ClearFeature as u8 =>
                Some(StandardRequest::ClearFeature),
            x if x == StandardRequest::GetConfiguration as u8 =>
                Some(StandardRequest::GetConfiguration),
            x if x == StandardRequest::GetDescriptor as u8 =>
                Some(StandardRequest::GetDescriptor),
            x if x == StandardRequest::GetInterface as u8 =>
                Some(StandardRequest::GetInterface),
            x if x == StandardRequest::GetStatus as u8 =>
                Some(StandardRequest::GetStatus),
            x if x == StandardRequest::SetAddress as u8 =>
                Some(StandardRequest::SetAddress),
            x if x == StandardRequest::SetConfiguration as u8 =>
                Some(StandardRequest::SetConfiguration),
            x if x == StandardRequest::SetDescriptor as u8 =>
                Some(StandardRequest::SetDescriptor),
            x if x == StandardRequest::SetFeature as u8 =>
                Some(StandardRequest::SetFeature),
            x if x == StandardRequest::SetInterface as u8 =>
                Some(StandardRequest::SetInterface),
            x if x == StandardRequest::SynchFrame as u8 =>
                Some(StandardRequest::SynchFrame),
            _ => None,
        }
    }
}

impl VendorRequest {
    /// Attempt to convert a u8 to a VendorRequest
    pub fn from_u8(x: u8) -> Option<Self> {
        match x {
            x if x == VendorRequest::SetCS as u8 =>
                Some(VendorRequest::SetCS),
            x if x == VendorRequest::SetFPGA as u8 =>
                Some(VendorRequest::SetFPGA),
            x if x == VendorRequest::SetMode as u8 =>
                Some(VendorRequest::SetMode),
            x if x == VendorRequest::SetTPwr as u8 =>
                Some(VendorRequest::SetTPwr),
            x if x == VendorRequest::GetTPwr as u8 =>
                Some(VendorRequest::GetTPwr),
            x if x == VendorRequest::SetLED as u8 =>
                Some(VendorRequest::SetLED),
            x if x == VendorRequest::Bootload as u8 =>
                Some(VendorRequest::Bootload),
            _ => None,
        }
    }
}

/// Trait for structs which can be safely cast to &[u8].
///
/// Traits implementing ToBytes must be repr(packed).
pub unsafe trait ToBytes: Sized {
    fn to_bytes(&self) -> &[u8] {
        // UNSAFE: We return a non-mutable slice into this packed struct's
        // memory at the length of the struct, with a lifetime bound to &self.
        unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8,
                                        core::mem::size_of::<Self>())
        }
    }
}

unsafe impl ToBytes for DeviceDescriptor {}
unsafe impl ToBytes for ConfigurationDescriptor {}
unsafe impl ToBytes for InterfaceDescriptor {}
unsafe impl ToBytes for EndpointDescriptor {}
unsafe impl ToBytes for StringDescriptor {}
