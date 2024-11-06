Library for reading and writing PCX images in Rust
==================================================

Example usage:

```Rust
let mut reader = pcx::Reader::from_file("test-data/marbles.pcx").unwrap();
println!("width = {}, height = {}", reader.width(), reader.height());

let mut buffer = vec![0; reader.width() as usize * reader.height() as usize * 3];
reader.read_rgb_pixels(&mut buffer).unwrap();
```

See [API documentation](https://docs.rs/pcx/) for more info.


License
=======

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   https://opensource.org/licenses/MIT)
 * WTFPL license ([LICENSE-WTFPL](LICENSE-WTFPL) or http://www.wtfpl.net/about)

at your option.

Note that these licenses do not cover the test images (`test-data` folder).
