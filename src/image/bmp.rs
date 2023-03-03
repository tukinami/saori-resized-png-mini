use std::fs::File;
use std::io::prelude::Read;
use std::path::PathBuf;

use embedded_graphics::{pixelcolor::Rgb888, prelude::RgbColor, Pixel};
use tinybmp::Bmp;

use crate::error::ResizedPngError;

use super::ImageData;

pub(crate) fn read_image_data(path: &PathBuf) -> Result<ImageData, ResizedPngError> {
    let mut fs = File::open(path)?;
    let mut bytes = Vec::new();
    fs.read_to_end(&mut bytes)?;

    let bmp = Bmp::<Rgb888>::from_slice(&bytes)?;

    let mut buf = Vec::new();
    for Pixel(_position, color) in bmp.pixels() {
        buf.push(color.r());
        buf.push(color.g());
        buf.push(color.b());
        buf.push(u8::MAX);
    }

    let header = bmp.as_raw().header();
    let width = header.image_size.width;
    let height = header.image_size.height;

    Ok((buf, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod read_image_data {
        use super::*;

        #[test]
        fn success_when_valid_bmp_path() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.bmp");

            let (_data, width, height) = read_image_data(&path).unwrap();

            assert_eq!(width, 100);
            assert_eq!(height, 200);
        }

        #[test]
        fn failed_when_invalid_bmp_path() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.png");

            assert!(read_image_data(&path).is_err());
        }
    }
}
