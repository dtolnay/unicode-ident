#![allow(clippy::type_complexity, clippy::uninlined_format_args)]

use image::{ImageBuffer, Rgb};
use std::process;

fn main() {
    let width = 512;
    let height = 400;
    let diagrams: [(&str, fn(char) -> bool); 2] = [
        ("xid_start.png", unicode_ident::is_xid_start),
        ("xid_continue.png", unicode_ident::is_xid_continue),
    ];
    for (name, f) in diagrams {
        let mut imgbuf = ImageBuffer::new(width, height);
        for (col, row, pixel) in imgbuf.enumerate_pixels_mut() {
            *pixel = if char::from_u32(row * width + col).is_some_and(f) {
                Rgb([0u8, 0, 0])
            } else {
                Rgb([255, 255, 255])
            };
        }
        if let Err(err) = imgbuf.save(name) {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    }
}
