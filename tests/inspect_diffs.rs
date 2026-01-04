#![cfg(feature = "images")]

use image::io::Reader as ImageReader;
use image::GenericImageView;
use std::path::Path;

#[test]
fn inspect_saved_diffs() {
    let dir = Path::new("target/test-diffs");
    let files = ["ref_0.png", "crop_0.png", "diff_0.png"];
    for f in files.iter() {
        let p = dir.join(f);
        if !p.exists() {
            eprintln!("File {:?} not found", p);
            continue;
        }
        let img = ImageReader::open(&p)
            .expect("open img")
            .decode()
            .expect("decode");
        let rgb = img.to_rgb8();
        let pixels: Vec<_> = rgb.pixels().collect();
        let n = pixels.len() as f64;
        let mut sum = [0u64, 0u64, 0u64];
        for px in pixels.iter() {
            sum[0] += px.0[0] as u64;
            sum[1] += px.0[1] as u64;
            sum[2] += px.0[2] as u64;
        }
        let avg = [sum[0] as f64 / n, sum[1] as f64 / n, sum[2] as f64 / n];
        println!("{} avg={:?} size={:?}", p.display(), avg, rgb.dimensions());
    }
}
