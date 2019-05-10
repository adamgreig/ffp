use std::time::{Instant, Duration};

fn main() {
    let context = libusb::Context::new().unwrap();
    for device in context.devices().unwrap().iter() {
        let dd = device.device_descriptor().unwrap();
        if dd.vendor_id() == 0x1209 && dd.product_id() == 0x0001 {
            let mut handle = device.open().unwrap();
            handle.claim_interface(0).unwrap();
            let timeout = Duration::from_millis(200);
            let languages = dbg!(handle.read_languages(timeout).unwrap());
            let language = languages[0];
            println!("Opened device {:03}.{:03}: {}, SN {}, Manufacturer {}",
                     device.bus_number(), device.address(),
                     handle.read_product_string(language, &dd, timeout).unwrap(),
                     handle.read_serial_number_string(language, &dd, timeout).unwrap(),
                     handle.read_manufacturer_string(language, &dd, timeout).unwrap());
            const CHUNK_SIZE: usize = 64;
            let tx = [0u8; CHUNK_SIZE];
            let mut rx = [0u8; CHUNK_SIZE];
            let t0 = Instant::now();
            let n_chunks = 2048;
            for idx in 0..n_chunks {
                let ntx = handle.write_bulk(0x01, &tx, timeout).unwrap();
                let nrx = handle.read_bulk(0x81, &mut rx, timeout).unwrap();
                if ntx != tx.len() {
                    println!("Chunk {}: Only TX {} bytes", idx, ntx);
                }
                if nrx != tx.len() {
                    println!("Chunk {}: Only RX {} bytes", idx, ntx);
                }
            }
            let t_elapsed = (t0.elapsed().as_millis() as f64) / 1000.0;
            let bps = (CHUNK_SIZE * n_chunks * 8) as f64 / t_elapsed;
            let mbps = bps / (1024.0 * 1024.0);
            println!("128kB transferred and read back in {:.03}s, {:.02}Mbps", t_elapsed, mbps);
            break;
        }
    }
}
