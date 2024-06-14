use std::{fs::File, io::BufReader, path::PathBuf};

use image_webp::{DecodingError, WebPDecoder};

use crate::error::ResizedPngError;

use super::ImageData;

impl From<DecodingError> for ResizedPngError {
    fn from(value: DecodingError) -> Self {
        match value {
            DecodingError::IoError(_) => ResizedPngError::IoError,
            _ => ResizedPngError::DecodingError,
        }
    }
}

pub(crate) fn read_image_data(path: &PathBuf) -> Result<ImageData, ResizedPngError> {
    let fs = File::open(path)?;
    let buf_reader = BufReader::new(fs);
    let mut decoder = WebPDecoder::new(buf_reader)?;

    let output_buffer_size = decoder
        .output_buffer_size()
        .ok_or(ResizedPngError::Unsupported)?;
    let mut buffer = vec![0; output_buffer_size];

    decoder.read_image(&mut buffer)?;

    if !decoder.has_alpha() {
        buffer = buffer.chunks(3).fold(Vec::new(), |mut acc, item| {
            acc.extend(item);
            acc.push(u8::MAX);
            acc
        });
    }
    let (width, height) = decoder.dimensions();

    Ok((buffer, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod read_image_data {
        use super::*;

        #[test]
        fn success_when_valid_webp_path() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.webp");

            let (_data, width, height) = read_image_data(&path).unwrap();

            assert_eq!(width, 100);
            assert_eq!(height, 200);
        }

        #[test]
        fn failed_when_invalid_webp_path() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.png");

            assert!(read_image_data(&path).is_err());
        }
    }
}
