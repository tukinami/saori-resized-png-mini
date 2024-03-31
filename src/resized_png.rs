use std::{num::NonZeroU32, path::PathBuf};

use rgb::FromSlice;

use crate::error::ResizedPngError;
use crate::image;

pub(crate) fn get_image_type(src_path: &PathBuf) -> &'static str {
    if image::png::read_image_data(src_path).is_ok() {
        return "PNG";
    }
    if image::bmp::read_image_data(src_path).is_ok() {
        return "BMP";
    }
    if image::gif::read_image_data(src_path).is_ok() {
        return "GIF";
    }
    if image::jpeg::read_image_data(src_path).is_ok() {
        return "JPEG";
    }
    if image::webp::read_image_data(src_path).is_ok() {
        return "WEBP";
    }

    "UNKNOWN"
}

pub(crate) fn to_resized_png(
    src_path: &PathBuf,
    dist_path: &PathBuf,
    width_command: i64,
    height_command: i64,
) -> Result<(), ResizedPngError> {
    let (src_rgba, input_width_raw, input_height_raw) = image::png::read_image_data(src_path)
        .or(image::bmp::read_image_data(src_path))
        .or(image::gif::read_image_data(src_path))
        .or(image::jpeg::read_image_data(src_path))
        .or(image::webp::read_image_data(src_path))?;

    let (input_width, input_height) = NonZeroU32::new(input_width_raw)
        .zip(NonZeroU32::new(input_height_raw))
        .ok_or(ResizedPngError::InputSizeError)?;

    // サイズが計算できないときは、何もせず終了。
    let (output_width, output_height) =
        match output_size(width_command, height_command, input_width, input_height) {
            Some(v) => v,
            None => return Ok(()),
        };

    let mut dist_rgba = vec![0; (output_width.get() * output_height.get() * 4) as usize];

    let mut resizer = resize::new(
        input_width.get() as usize,
        input_height.get() as usize,
        output_width.get() as usize,
        output_height.get() as usize,
        resize::Pixel::RGBA8P,
        resize::Type::Lanczos3,
    )?;

    resizer.resize(src_rgba.as_rgba(), dist_rgba.as_rgba_mut())?;

    image::png::write_png(
        dist_path,
        &dist_rgba,
        output_width.get(),
        output_height.get(),
    )?;

    Ok(())
}

fn output_size(
    width_command: i64,
    height_command: i64,
    input_width: NonZeroU32,
    input_height: NonZeroU32,
) -> Option<(NonZeroU32, NonZeroU32)> {
    // 両方とも0未満ならサイズなし。
    if width_command < 0 && height_command < 0 {
        return None;
    }

    // command が0の場合は元のサイズが指定されているとして扱う。
    let width_origin = match width_command {
        0 => input_width.get() as i64,
        w => w,
    };
    let height_origin = match height_command {
        0 => input_height.get() as i64,
        h => h,
    };

    // originが0未満の場合はもう片方の拡大率に従う。
    let width_temp = match width_origin {
        w if w < 0 => {
            let ratio = height_origin as f64 / input_height.get() as f64;

            (input_width.get() as f64 * ratio) as u32
        }
        w => w as u32,
    };
    let height_temp = match height_origin {
        h if h < 0 => {
            let ratio = width_origin as f64 / input_width.get() as f64;

            (input_height.get() as f64 * ratio) as u32
        }
        h => h as u32,
    };

    // tempが0の場合は1にfallbackして返す。
    let width = NonZeroU32::new(width_temp).unwrap_or(NonZeroU32::new(1).unwrap());
    let height = NonZeroU32::new(height_temp).unwrap_or(NonZeroU32::new(1).unwrap());

    Some((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod get_image_type {
        use super::*;

        #[test]
        fn checking_value_when_image_file_exists() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.png");

            assert_eq!(get_image_type(&path), "PNG");
        }

        #[test]
        fn checking_value_when_non_image_file_exists() {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");

            assert_eq!(get_image_type(&path), "UNKNOWN");
        }

        #[test]
        fn checking_value_when_file_does_not_exist() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/something_wrong.png");
            assert_eq!(get_image_type(&path), "UNKNOWN");
        }
    }

    mod to_resized_png {
        use super::*;

        use tempfile::tempdir;

        #[test]
        fn success_when_input_image_is_png() {
            let out_dir = tempdir().unwrap();

            let src_path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.png");
            let dist_path = out_dir.path().join("from_png.png");
            let width_command = 50;
            let height_command = 100;

            to_resized_png(&src_path, &dist_path, width_command, height_command).unwrap();

            assert!(dist_path.exists());

            out_dir.close().unwrap();
        }

        #[test]
        fn success_when_input_image_is_webp() {
            let out_dir = tempdir().unwrap();

            let src_path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.webp");
            let dist_path = out_dir.path().join("from_webp.png");
            let width_command = -1;
            let height_command = 50;

            to_resized_png(&src_path, &dist_path, width_command, height_command).unwrap();

            assert!(dist_path.exists());

            out_dir.close().unwrap();
        }

        #[test]
        fn success_when_input_image_is_bmp() {
            let out_dir = tempdir().unwrap();

            let src_path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.bmp");
            let dist_path = out_dir.path().join("from_bmp.png");
            let width_command = 50;
            let height_command = -1;

            to_resized_png(&src_path, &dist_path, width_command, height_command).unwrap();

            assert!(dist_path.exists());

            out_dir.close().unwrap();
        }

        #[test]
        fn success_when_input_image_is_jpg() {
            let out_dir = tempdir().unwrap();

            let src_path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.jpg");
            let dist_path = out_dir.path().join("from_jpg.png");
            let width_command = 0;
            let height_command = 0;

            to_resized_png(&src_path, &dist_path, width_command, height_command).unwrap();

            assert!(dist_path.exists());

            out_dir.close().unwrap();
        }
    }

    mod output_size {
        use super::*;

        #[test]
        fn none_when_both_width_and_height_are_minus() {
            let width_command = -1;
            let height_command = -1;
            let input_width = NonZeroU32::new(100).unwrap();
            let input_height = NonZeroU32::new(200).unwrap();

            assert!(
                output_size(width_command, height_command, input_width, input_height).is_none()
            );
        }

        #[test]
        fn original_value_when_width_and_height_are_0() {
            let width_command = 0;
            let height_command = 0;
            let input_width = NonZeroU32::new(100).unwrap();
            let input_height = NonZeroU32::new(200).unwrap();

            let (width, height) =
                output_size(width_command, height_command, input_width, input_height).unwrap();

            assert_eq!(width, input_width);
            assert_eq!(height, input_height);
        }

        #[test]
        fn keep_aspect_ratio_when_one_of_width_and_height_is_minus() {
            let width_command = -1;
            let height_command = 100;
            let input_width = NonZeroU32::new(100).unwrap();
            let input_height = NonZeroU32::new(200).unwrap();

            let (width, height) =
                output_size(width_command, height_command, input_width, input_height).unwrap();

            assert_eq!(width, NonZeroU32::new(50).unwrap());
            assert_eq!(height, NonZeroU32::new(100).unwrap());
        }

        #[test]
        fn target_values_when_width_and_henght_are_not_0_and_minis() {
            let width_command = 200;
            let height_command = 300;
            let input_width = NonZeroU32::new(100).unwrap();
            let input_height = NonZeroU32::new(200).unwrap();

            let (width, height) =
                output_size(width_command, height_command, input_width, input_height).unwrap();

            assert_eq!(width, NonZeroU32::new(200).unwrap());
            assert_eq!(height, NonZeroU32::new(300).unwrap());
        }
    }
}
