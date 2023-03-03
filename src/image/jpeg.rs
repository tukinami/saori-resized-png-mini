use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use jpeg_decoder::{Decoder, PixelFormat};

use crate::error::ResizedPngError;

use super::ImageData;

pub(crate) fn read_image_data(path: &PathBuf) -> Result<ImageData, ResizedPngError> {
    let fs = File::open(path)?;
    let mut decoder = Decoder::new(BufReader::new(fs));
    let raw_pixels = decoder.decode()?;
    let metadata = decoder.info().expect("decoding already finished");

    let buf = to_rgb(&raw_pixels, &metadata.pixel_format)?;
    let width = metadata.width as u32;
    let height = metadata.height as u32;

    Ok((buf, width, height))
}

fn to_rgb(raw_pixels: &[u8], pixel_format: &PixelFormat) -> Result<Vec<u8>, ResizedPngError> {
    match pixel_format {
        PixelFormat::L8 => Ok(raw_pixels
            .iter()
            .flat_map(|v| [*v, *v, *v, u8::MAX])
            .collect()),
        PixelFormat::L16 => Ok(raw_pixels
            .iter()
            .step_by(2)
            .flat_map(|v| [*v, *v, *v, u8::MAX])
            .collect()),
        PixelFormat::RGB24 => {
            let mut pixels = Vec::new();

            let mut iter = raw_pixels.iter();
            while let (Some(r), Some(g), Some(b)) = (iter.next(), iter.next(), iter.next()) {
                pixels.push(*r);
                pixels.push(*g);
                pixels.push(*b);
                pixels.push(u8::MAX);
            }

            Ok(pixels)
        }
        PixelFormat::CMYK32 => Err(ResizedPngError::Unsupported),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod read_image_data {
        use super::*;

        #[test]
        fn success_when_valid_jpg_path() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.jpg");

            let (_data, width, height) = read_image_data(&path).unwrap();

            assert_eq!(width, 100);
            assert_eq!(height, 200);
        }

        #[test]
        fn failed_when_invalid_jpg_path() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.png");

            assert!(read_image_data(&path).is_err());
        }
    }

    mod to_rgb {
        use super::*;

        #[test]
        fn success_when_valid_l8_buffer() {
            let buf = [0, 1, 2];
            let pixel_format = PixelFormat::L8;

            let pixels = to_rgb(&buf, &pixel_format).unwrap();

            assert_eq!(
                pixels,
                vec![0, 0, 0, u8::MAX, 1, 1, 1, u8::MAX, 2, 2, 2, u8::MAX]
            );
        }

        #[test]
        fn success_when_valid_l16_buffer() {
            let buf = [0, 1, 2, 3];
            let pixel_format = PixelFormat::L16;

            let pixels = to_rgb(&buf, &pixel_format).unwrap();

            assert_eq!(pixels, vec![0, 0, 0, u8::MAX, 2, 2, 2, u8::MAX]);
        }

        #[test]
        fn success_when_valid_rgb_buffer() {
            let buf = [0, 1, 2];
            let pixel_format = PixelFormat::RGB24;

            let pixels = to_rgb(&buf, &pixel_format).unwrap();

            assert_eq!(pixels, vec![0, 1, 2, u8::MAX]);
        }

        #[test]
        fn failed_when_cmyk_buffer() {
            let buf = [0, 1, 2, 3];
            let pixel_format = PixelFormat::CMYK32;

            assert!(to_rgb(&buf, &pixel_format).is_err());
        }
    }
}
