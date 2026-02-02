#![no_main]
use libfuzzer_sys::fuzz_target;
use pcx::tests::{round_trip_paletted, round_trip_rgb};

fuzz_target!(|data: &[u8]| {
    let Some((size, data)) = data.split_at_checked(5) else {
        return;
    };
    let width = size[0] as u16 * 256 + size[1] as u16;
    let height = size[2] as u16 * 256 + size[3] as u16;
    let palette_size = size[4];

    if width == 0 || height == 0 || width == 0xFFFF || width as usize * height as usize > (10 << 16)
    {
        return;
    }

    if palette_size > 0 {
        let size = width as usize * height as usize;
        if let Some((palette, data)) = data.split_at_checked(palette_size as usize * 3) {
            let pixels = pad_to_size(size, data);
            round_trip_paletted(width, height, &palette, &pixels);
        }
    } else {
        let size = width as usize * height as usize * 3;
        let pixels = pad_to_size(size, data);
        round_trip_rgb(width, height, &pixels);
    }
});

fn pad_to_size(size: usize, data: &[u8]) -> Vec<u8> {
    let mut pixels = vec![0; size];
    let data_len = data.len().min(size);
    pixels[..data_len].copy_from_slice(&data[..data_len]);
    pixels
}
