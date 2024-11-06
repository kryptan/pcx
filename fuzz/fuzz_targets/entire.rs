#![no_main]
use libfuzzer_sys::fuzz_target;
use pcx::Reader;

fuzz_target!(|data: &[u8]| {
    let Ok(mut pcx) = Reader::from_mem(data) else {
        return;
    };

    let size = pcx.width() as usize * pcx.height() as usize * 3;
    if size > 5000 {
        return;
    }

    let mut buffer = vec![0; size];
    let _ = pcx.read_rgb_pixels(&mut buffer);
});
