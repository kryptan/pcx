use std::path::Path;
use std::fs::File;
use std::{io, iter};
use walkdir::WalkDir;
use std::path;
use image;

use Reader;

fn test_file(path : &Path) {
    print!("{} ", path.display());

    let bmp_path = path.with_extension("png");
    let bmp_file = File::open(bmp_path).unwrap();
    let reference_image = image::load(io::BufReader::new(bmp_file), image::ImageFormat::PNG).unwrap();
    let reference_image = reference_image.to_rgb();

    let mut pcx = Reader::new_from_file(path).unwrap();
    assert_eq!(pcx.width() as u32, reference_image.width());
    assert_eq!(pcx.height() as u32, reference_image.height());

    if pcx.is_paletted() {
        print!("paletted ");
        let mut image = Vec::new();
        for i in 0..pcx.height() {
            let mut row : Vec<u8> = iter::repeat(0).take(pcx.width() as usize).collect();
        //    println!("{:?}", i);
          /*  if i == pcx.height() - 1 {
                println!("aha");
            }*/
            pcx.next_row_paletted(&mut row).unwrap();
          //  println!("{:?}", row);
            image.push(row);
        }

     //   let out = File::create("C:/projects/pcx/out.csv").unwrap();
   //     let mut out = io::BufWriter::new(out);

     //   println!("{:?}", pcx.header);

        let mut palette = [0; 256*3];
        let pl = pcx.read_palette(&mut palette).unwrap();

  /*      println!("{:?}", pl);

        println!("{:?}", &palette[..]);

        writeln!(out, "{{");
        for y in 0..reference_image.height() {
            writeln!(out, "{{");
            for x in 0..reference_image.width() {
                let i = image[y as usize][x as usize] as usize;
                let pcx_r = palette[i*3 + 0];
                let pcx_g = palette[i*3 + 1];
                let pcx_b = palette[i*3 + 2];
                write!(out, "{{{},{},{}}}{}", pcx_r, pcx_g, pcx_b, if x == reference_image.width() - 1 { "" } else { "," });
            }
            writeln!(out, "}}{}", if y == reference_image.height() - 1 { "" } else { "," });
        }
        writeln!(out, "}}");*/

        for y in 0..reference_image.height() {
            for x in 0..reference_image.width() {
                let i = image[y as usize][x as usize] as usize;
                let pcx_r = palette[i*3 + 0];
                let pcx_g = palette[i*3 + 1];
                let pcx_b = palette[i*3 + 2];

                let reference_pixel = reference_image.get_pixel(x as u32, y as u32);
                let reference_r = reference_pixel.data[0];
                let reference_g = reference_pixel.data[1];
                let reference_b = reference_pixel.data[2];

                assert_eq!(pcx_r, reference_r);
                assert_eq!(pcx_g, reference_g);
                assert_eq!(pcx_b, reference_b);
            }
        }
    } else {
        print!("not paletted ");
    }
    println!("- Ok.");
}

#[test]
fn samples() {
   // test_file(&path::PathBuf::from("C:/projects/pcx/gmarbles.pcx"));

    if let Some(samples_path) = option_env!("PCX_SAMPLES") {
        println!("Testing samples at {}", env!("PCX_SAMPLES"));
        for entry in WalkDir::new(samples_path) {
            let entry = entry.unwrap();

            if let Some(ext) = entry.path().extension() {
                let ext = ext.to_str().unwrap();
                if ext == "pcx" || ext == "PCX" {
                    test_file(entry.path())
                }
            }
        }

      //  assert!(false);
    }
}