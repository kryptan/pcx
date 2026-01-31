#![no_main]
use libfuzzer_sys::fuzz_target;
use pcx::{Reader, WriterRgb};

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let (size, data) = data.split_at(2);
    let width = size[0];
    let height = size[1];

    if width == 0 || height == 0 {
        return;
    }

    let size = width as usize * height as usize * 3;
    let mut written_pixels = vec![0; size];
    let data_len = data.len().min(size);
    written_pixels[..data_len].copy_from_slice(&data[..data_len]);

    let mut pcx = Vec::new();
    let mut writer = WriterRgb::new(&mut pcx, (width as u16, height as u16), (300, 300)).unwrap();
    let row_len = width as usize * 3;
    for y in 0..height {
        writer
            .write_row(&written_pixels[y as usize * row_len..(y as usize + 1) * row_len])
            .unwrap();
    }
    writer.finish().unwrap();

    let mut reader = Reader::from_mem(&pcx).unwrap();
    assert_eq!(reader.width(), width as u16);
    assert_eq!(reader.height(), height as u16);

    let mut read_pixels = vec![0; size];
    reader.read_rgb_pixels(&mut read_pixels).unwrap();
    assert!(written_pixels == read_pixels);
});
