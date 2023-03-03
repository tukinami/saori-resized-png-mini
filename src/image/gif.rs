use std::fs::File;
use std::path::PathBuf;

use crate::error::ResizedPngError;

use super::ImageData;

pub(crate) fn read_image_data(path: &PathBuf) -> Result<ImageData, ResizedPngError> {
    let mut decode_options = gif::DecodeOptions::new();
    decode_options.set_color_output(gif::ColorOutput::RGBA);

    let fs = File::open(path)?;
    let mut decoder = decode_options.read_info(fs)?;

    let frame = decoder
        .read_next_frame()?
        .ok_or(ResizedPngError::DecodingError)?;

    let buf = frame.buffer.to_vec();
    let width = frame.width as u32;
    let height = frame.height as u32;

    Ok((buf, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod read_image_data {
        use super::*;

        #[test]
        fn success_when_valid_gif_path() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.gif");

            let (_data, width, height) = read_image_data(&path).unwrap();

            assert_eq!(width, 100);
            assert_eq!(height, 200);
        }

        #[test]
        fn failed_when_invalid_gif_path() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.png");

            assert!(read_image_data(&path).is_err());
        }
    }
}
