// Copyright 2019-2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use core::convert::TryFrom;
use num_enum::TryFromPrimitive;
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

#[derive(TryFromPrimitive)]
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

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum VendorRequest {
    SetCS = 1,
    SetFPGA = 2,
    SetMode = 3,
    SetTPwr = 4,
    GetTPwr = 5,
    SetLED = 6,
    Bootload = 7,
    GetOSFeature = b'A',
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum DescriptorType {
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
    HID = 0x21,
    HIDReport = 0x22,
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
    pub bString: [u8; 62],
}

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
pub struct HIDDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bcdHID: u16,
    pub bCountryCode: u8,
    pub bNumDescriptors: u8,
    pub bSubDescriptorType: u8,
    pub wSubDescriptorLength: u16,
}

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
pub struct MSCompatibleIDDescriptor {
    pub dwLength: u32,
    pub bcdVersion: u16,
    pub wIndex: u16,
    pub bNumSections: u8,
    pub _rsvd0: [u8; 7],
    pub features: [MSCompatibleIDDescriptorFunction; 2],
}

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
pub struct MSCompatibleIDDescriptorFunction {
    pub bInterfaceNumber: u8,
    pub _rsvd0: u8,
    pub sCompatibleID: [u8; 8],
    pub sSubCompatibleID: [u8; 8],
    pub _rsvd1: [u8; 6],
}

#[allow(non_snake_case)]
pub struct MSPropertiesOSDescriptor {
    pub bcdVersion: u16,
    pub wIndex: u16,
    pub wCount: u16,
    pub features: [MSPropertiesOSDescriptorFeature; 1],
}

#[allow(non_snake_case)]
pub struct MSPropertiesOSDescriptorFeature {
    pub dwPropertyDataType: u32,
    pub bPropertyName: &'static str,
    pub bPropertyData: &'static str,
}

#[allow(non_snake_case)]
#[repr(u16)]
#[derive(TryFromPrimitive)]
pub enum OSFeatureDescriptorType {
    CompatibleID    = 4,
    Properties      = 5,
}

#[allow(non_camel_case_types)]
#[allow(unused)]
#[repr(u32)]
pub enum MSPropertyDataType {
    REG_SZ                      = 1,
    REG_EXPAND_SZ               = 2,
    REG_BINARY                  = 3,
    REG_DWORD_LITTLE_ENDIAN     = 4,
    REG_DWORD_BIG_ENDIAN        = 5,
    REG_LINK                    = 6,
    REG_MULTI_SZ                = 7,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
#[allow(unused)]
pub enum SetupDirection {
    HostToDevice = 0,
    DeviceToHost = 1,
}

#[derive(PartialEq,TryFromPrimitive)]
#[repr(u8)]
pub enum SetupType {
    Standard = 0,
    Class = 1,
    Vendor = 2,
    Reserved = 3,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
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
        let x = (self.bmRequestType & (0b1 << 7)) >> 5;
        SetupDirection::try_from(x).unwrap()
    }

    pub fn setup_type(&self) -> SetupType {
        let x = (self.bmRequestType & (0b11 << 5)) >> 5;
        SetupType::try_from(x).unwrap()
    }

    #[allow(unused)]
    pub fn setup_recipient(&self) -> SetupRecipient {
        let x = self.bmRequestType & 0b11111;
        SetupRecipient::try_from(x).unwrap_or(SetupRecipient::Unknown)
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
unsafe impl ToBytes for HIDDescriptor {}
unsafe impl ToBytes for MSCompatibleIDDescriptor {}

impl MSPropertiesOSDescriptor {
    /// Retrieve the total length of a MSPropertiesOSDescriptor,
    /// including the length of variable string contents once UTF-16 encoded.
    pub fn len(&self) -> usize {
        // Header section
        let mut len = 10;

        for feature in self.features.iter() {
            len += feature.len();
        }

        len
    }

    /// Write descriptor contents into a provided &mut [u8], which must
    /// be at least self.len() long.
    pub fn write_to_buf(&self, buf: &mut [u8]) {
        let len = self.len() as u32;
        buf[0..4].copy_from_slice(&len.to_le_bytes());
        buf[4..6].copy_from_slice(&self.bcdVersion.to_le_bytes());
        buf[6..8].copy_from_slice(&self.wIndex.to_le_bytes());
        buf[8..10].copy_from_slice(&self.wCount.to_le_bytes());
        let mut i = 10;

        for feature in self.features.iter() {
            feature.write_to_buf(&mut buf[i..]);
            i += feature.len();
        }
    }
}

impl MSPropertiesOSDescriptorFeature {
    pub fn len(&self) -> usize {
        // Fixed length parts of feature
        let mut len = 14;

        // String parts
        len += self.name_len();
        len += self.data_len();

        len
    }

    fn name_len(&self) -> usize {
        self.bPropertyName.encode_utf16().count() * 2
    }

    fn data_len(&self) -> usize {
        self.bPropertyData.encode_utf16().count() * 2
    }

    pub fn write_to_buf(&self, buf: &mut [u8]) {
        let len = self.len() as u32;
        let name_len = self.name_len() as u16;
        let data_len = self.data_len() as u32;
        buf[0..4].copy_from_slice(&len.to_le_bytes());
        buf[4..8].copy_from_slice(&self.dwPropertyDataType.to_le_bytes());
        buf[8..10].copy_from_slice(&name_len.to_le_bytes());
        let mut i = 10;
        for cp in self.bPropertyName.encode_utf16() {
            let [u1, u2] = cp.to_le_bytes();
            buf[i  ] = u1;
            buf[i+1] = u2;
            i += 2;
        }
        buf[i..i+4].copy_from_slice(&data_len.to_le_bytes());
        i += 4;
        for cp in self.bPropertyData.encode_utf16() {
            let [u1, u2] = cp.to_le_bytes();
            buf[i  ] = u1;
            buf[i+1] = u2;
            i += 2;
        }
    }
}
