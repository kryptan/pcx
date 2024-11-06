use image;
use std::fs::File;
use std::path::Path;
use std::{io, iter};
use walkdir::WalkDir;

use crate::Reader;

#[derive(Eq, PartialEq)]
enum ReadKind {
    Separate,
    Interleaved,
    Entire,
}

fn test_file(path: &Path, kind: ReadKind) {
    print!("{} ", path.display());

    let png_path = path.with_extension("png");
    let png_file = File::open(png_path).unwrap();
    let reference_image =
        image::load(io::BufReader::new(png_file), image::ImageFormat::PNG).unwrap();
    let reference_image = reference_image.to_rgb();

    let mut pcx = Reader::from_file(path).unwrap();
    assert_eq!(pcx.width() as u32, reference_image.width());
    assert_eq!(pcx.height() as u32, reference_image.height());

    if kind == ReadKind::Entire {
        let mut buffer = vec![0; pcx.width() as usize * pcx.height() as usize * 3];
        pcx.read_rgb_pixels(&mut buffer).unwrap();

        for y in 0..reference_image.height() {
            for x in 0..reference_image.width() {
                let index = ((y as usize * pcx.width() as usize) + x as usize) * 3;
                let reference = reference_image.get_pixel(x as u32, y as u32);

                //dbg!((x, y));

                assert_eq!(buffer[index + 0], reference.data[0]);
                assert_eq!(buffer[index + 1], reference.data[1]);
                assert_eq!(buffer[index + 2], reference.data[2]);
            }
        }
        return;
    }

    if pcx.is_paletted() {
        print!("paletted ");

        let mut palette = [0; 256 * 3];
        pcx.get_palette(&mut palette).unwrap();

        let mut image = Vec::new();
        for _ in 0..pcx.height() {
            let mut row: Vec<u8> = iter::repeat(0).take(pcx.width() as usize).collect();
            pcx.next_row_paletted(&mut row).unwrap();
            image.push(row);
        }

        for y in 0..reference_image.height() {
            for x in 0..reference_image.width() {
                let i = image[y as usize][x as usize] as usize;
                let reference = reference_image.get_pixel(x as u32, y as u32);

                assert_eq!(palette[i * 3 + 0], reference.data[0]);
                assert_eq!(palette[i * 3 + 1], reference.data[1]);
                assert_eq!(palette[i * 3 + 2], reference.data[2]);
            }
        }
    } else if kind == ReadKind::Interleaved {
        print!("not paletted ");

        let mut image = Vec::new();
        for _ in 0..pcx.height() {
            let mut rgb: Vec<u8> = iter::repeat(0).take((pcx.width() as usize) * 3).collect();
            pcx.next_row_rgb(&mut rgb).unwrap();
            image.push(rgb);
        }

        for y in 0..reference_image.height() {
            for x in 0..reference_image.width() {
                let pcx_r = image[y as usize][(x as usize) * 3 + 0];
                let pcx_g = image[y as usize][(x as usize) * 3 + 1];
                let pcx_b = image[y as usize][(x as usize) * 3 + 2];

                let reference = reference_image.get_pixel(x as u32, y as u32);

                assert_eq!(pcx_r, reference.data[0]);
                assert_eq!(pcx_g, reference.data[1]);
                assert_eq!(pcx_b, reference.data[2]);
            }
        }
    } else {
        print!("not paletted ");

        let mut image_r = Vec::new();
        let mut image_g = Vec::new();
        let mut image_b = Vec::new();
        for _ in 0..pcx.height() {
            let mut r: Vec<u8> = iter::repeat(0).take(pcx.width() as usize).collect();
            let mut g: Vec<u8> = iter::repeat(0).take(pcx.width() as usize).collect();
            let mut b: Vec<u8> = iter::repeat(0).take(pcx.width() as usize).collect();
            pcx.next_row_rgb_separate(&mut r, &mut g, &mut b).unwrap();
            image_r.push(r);
            image_g.push(g);
            image_b.push(b);
        }

        for y in 0..reference_image.height() {
            for x in 0..reference_image.width() {
                let pcx_r = image_r[y as usize][x as usize];
                let pcx_g = image_g[y as usize][x as usize];
                let pcx_b = image_b[y as usize][x as usize];

                let reference_pixel = reference_image.get_pixel(x as u32, y as u32);
                let reference_r = reference_pixel.data[0];
                let reference_g = reference_pixel.data[1];
                let reference_b = reference_pixel.data[2];

                assert_eq!(pcx_r, reference_r);
                assert_eq!(pcx_g, reference_g);
                assert_eq!(pcx_b, reference_b);
            }
        }
    }
    println!("- Ok.");
}

fn test_files(path: &str) {
    println!("Testing samples at {}", path);
    for entry in WalkDir::new(path) {
        let entry = entry.unwrap();

        if let Some(ext) = entry.path().extension() {
            let ext = ext.to_str().unwrap();
            if ext == "pcx" || ext == "PCX" {
                test_file(entry.path(), ReadKind::Interleaved);
                test_file(entry.path(), ReadKind::Separate);
                test_file(entry.path(), ReadKind::Entire);
            }
        }
    }
}

#[test]
fn samples() {
    let samples_path = env!("CARGO_MANIFEST_DIR").to_string() + "/test-data";
    test_files(&samples_path);
}

#[test]
fn samples_env() {
    if let Some(samples_path) = option_env!("PCX_RS_SAMPLES") {
        test_files(samples_path);
    }
}
