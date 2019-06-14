// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

static HEX_DIGITS: [u8; 16] = [
    48, 49, 50, 51, 52, 53, 54, 55, 56, 57,
    97, 98, 99, 100, 101, 102,
];

/// Returns the 12-byte (96-bit) unique ID
pub fn get_id() -> [u8; 12] {
    // UNSAFE: Reads fixed memory address known to contain unqiue ID.
    unsafe { read_id() }
}

/// Returns the unique ID as ASCII hex
pub fn get_hex_id() -> [u8; 24] {
    let id = get_id();
    let mut out = [0u8; 24];
    for (idx, v) in id.iter().enumerate() {
        let v1 = v & 0x0F;
        let v2 = (v & 0xF0) >> 4;
        out[idx*2] = HEX_DIGITS[v1 as usize];
        out[idx*2+1] = HEX_DIGITS[v2 as usize];
    }
    out
}

unsafe fn read_id() -> [u8; 12] {
    let id1: [u8; 4] = (*(0x1FFF_F7AC as *const u32)).to_le_bytes();
    let id2: [u8; 4] = (*(0x1FFF_F7B0 as *const u32)).to_le_bytes();
    let id3: [u8; 4] = (*(0x1FFF_F7B4 as *const u32)).to_le_bytes();
    [
        id1[0], id1[1], id1[2], id1[3],
        id2[0], id2[1], id2[2], id2[3],
        id3[0], id3[1], id3[2], id3[3],
    ]
}
