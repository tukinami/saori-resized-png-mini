use core::slice;
use std::{fs::File, io::prelude::Read, path::PathBuf};

use libwebp_sys::{WebPDecodeRGBA, WebPGetInfo};

use crate::error::ResizedPngError;

use super::ImageData;

pub(crate) fn read_image_data(path: &PathBuf) -> Result<ImageData, ResizedPngError> {
    let mut fs = File::open(path)?;
    let mut bytes = Vec::new();
    fs.read_to_end(&mut bytes)?;

    let mut width = 0;
    let mut height = 0;
    let len = bytes.len();

    let is_webp = unsafe { WebPGetInfo(bytes.as_ptr(), len, &mut width, &mut height) };

    if is_webp == 0 {
        return Err(ResizedPngError::Unsupported);
    }

    let out_buf = unsafe { WebPDecodeRGBA(bytes.as_ptr(), len, &mut width, &mut height) };

    let data_raw = unsafe { slice::from_raw_parts(out_buf, (width * height * 4) as usize) };

    Ok((data_raw.to_vec(), width as u32, height as u32))
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
