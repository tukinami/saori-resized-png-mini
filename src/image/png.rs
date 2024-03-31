use std::io::BufWriter;
use std::path::PathBuf;
use std::{fs::File, slice::Iter};

use png::{BitDepth, ColorType, Decoder, Encoder, Info};

use crate::error::ResizedPngError;

use super::ImageData;

pub(crate) fn read_image_data(path: &PathBuf) -> Result<ImageData, ResizedPngError> {
    let fs = File::open(path)?;
    let decoder = Decoder::new(fs);
    let mut reader = decoder.read_info()?;

    let mut buf = vec![0; reader.output_buffer_size()];
    let output_info = reader.next_frame(&mut buf)?;
    let bytes = &buf[..output_info.buffer_size()];

    let info = reader.info();

    let result = buf_to_rgba(bytes, info)?;

    Ok((result, info.width, info.height))
}

pub(crate) fn write_png(
    path: &PathBuf,
    buf: &[u8],
    width: u32,
    height: u32,
) -> Result<(), ResizedPngError> {
    let fs = File::create(path)?;
    let w = &mut BufWriter::new(fs);

    let mut encoder = Encoder::new(w, width, height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);

    let mut writer = encoder.write_header()?;
    writer.write_image_data(buf)?;

    Ok(())
}

fn buf_to_rgba(raw_bytes: &[u8], info: &Info) -> Result<Vec<u8>, ResizedPngError> {
    let mut result = match info.color_type {
        ColorType::Grayscale => {
            let bytes = read_bytes_for_bit_depth_8(raw_bytes, info)?;

            bytes.iter().flat_map(|v| [*v, *v, *v, u8::MAX]).collect()
        }
        ColorType::GrayscaleAlpha => {
            let bytes = read_bytes_for_bit_depth_8(raw_bytes, info)?;

            let mut result = Vec::new();
            let mut bytes_iter = bytes.iter();

            while let (Some(g), Some(a)) = (bytes_iter.next(), bytes_iter.next()) {
                result.push(*g);
                result.push(*g);
                result.push(*g);
                result.push(*a);
            }

            result
        }
        ColorType::Rgb => {
            let bytes = read_bytes_for_bit_depth_8(raw_bytes, info)?;

            let mut result = Vec::new();
            let mut bytes_iter = bytes.iter();

            while let (Some(r), Some(g), Some(b)) =
                (bytes_iter.next(), bytes_iter.next(), bytes_iter.next())
            {
                result.push(*r);
                result.push(*g);
                result.push(*b);
                result.push(u8::MAX);
            }

            result
        }
        ColorType::Rgba => read_bytes_for_bit_depth_8(raw_bytes, info)?,
        ColorType::Indexed => {
            let indices = read_bytes_for_usize(raw_bytes, info)?;
            let palette = match &info.palette {
                Some(v) => split_palette(v)?,
                None => return Err(ResizedPngError::DecodingError),
            };

            let mut result = Vec::new();
            let mut indices_iter = indices.iter();
            let pixel_len = info.width as usize * info.height as usize;
            for _i in 0..pixel_len {
                let index = *indices_iter.next().ok_or(ResizedPngError::DecodingError)?;
                let target_palette = palette.get(index).ok_or(ResizedPngError::DecodingError)?;
                let alpha = info
                    .trns
                    .as_ref()
                    .and_then(|v| v.get(index).copied())
                    .unwrap_or(u8::MAX);

                result.push(target_palette[0]);
                result.push(target_palette[1]);
                result.push(target_palette[2]);
                result.push(alpha);
            }

            result
        }
    };

    let rgba_len = (info.width as usize * info.height as usize) * 4;

    if result.len() < rgba_len {
        Err(ResizedPngError::DecodingError)
    } else {
        result.resize(rgba_len, 0);
        Ok(result)
    }
}

fn read_bytes_for_bit_depth_8(buf: &[u8], info: &Info) -> Result<Vec<u8>, ResizedPngError> {
    let f = match &info.bit_depth {
        BitDepth::One => read_byte_for_bit_depth_8_when_bit_depth_one,
        BitDepth::Two => read_byte_for_bit_depth_8_when_bit_depth_two,
        BitDepth::Four => read_byte_for_bit_depth_8_when_bit_depth_four,
        BitDepth::Eight => return Ok(buf.to_vec()),
        BitDepth::Sixteen => return Ok(buf.iter().step_by(2).copied().collect()),
    };

    let mut result = Vec::new();
    let mut tmp = [0; 8];
    let mut buf_iter = buf.iter();

    let channel_len = info.color_type.samples();
    let width = info.width as usize;
    let height = info.height as usize;
    let line_length = width * channel_len;

    let mut line = Vec::new();
    for _ in 0..height {
        line.clear();

        for t in &mut buf_iter {
            let tmp_size = f(*t, &mut tmp);

            line.extend_from_slice(&tmp[..tmp_size]);

            if line.len() >= line_length {
                line.resize(line_length, 0);
                break;
            }
        }
        if line.len() < line_length {
            return Err(ResizedPngError::DecodingError);
        }

        result.extend_from_slice(&line);
    }

    Ok(result)
}

fn read_byte_for_bit_depth_8_when_bit_depth_one(t: u8, output: &mut [u8; 8]) -> usize {
    for (i, element) in output.iter_mut().enumerate().take(8) {
        *element = if (t << i) | 0b01111111 == u8::MAX {
            u8::MAX
        } else {
            0
        };
    }
    8
}

fn read_byte_for_bit_depth_8_when_bit_depth_two(t: u8, output: &mut [u8; 8]) -> usize {
    for (i, element) in output.iter_mut().enumerate().take(4) {
        let mut v = 0;
        if (t << (i * 2)) | 0b01111111 == u8::MAX {
            v += 0b10000000;
        }
        if (t << (i * 2 + 1)) | 0b01111111 == u8::MAX {
            v += 0b01111111;
        }

        *element = v;
    }
    4
}

fn read_byte_for_bit_depth_8_when_bit_depth_four(t: u8, output: &mut [u8; 8]) -> usize {
    for (i, element) in output.iter_mut().enumerate().take(2) {
        let mut v = 0;
        if (t << (i * 4)) | 0b01111111 == u8::MAX {
            v += 0b10000000;
        }
        if (t << (i * 4 + 1)) | 0b01111111 == u8::MAX {
            v += 0b01000000;
        }
        if (t << (i * 4 + 2)) | 0b01111111 == u8::MAX {
            v += 0b00100000;
        }
        if (t << (i * 4 + 3)) | 0b01111111 == u8::MAX {
            v += 0b00011111;
        }
        *element = v;
    }
    2
}

fn read_bytes_for_usize(buf: &[u8], info: &Info) -> Result<Vec<usize>, ResizedPngError> {
    let f = match &info.bit_depth {
        BitDepth::One => read_byte_for_usize_when_bit_depth_one,
        BitDepth::Two => read_byte_for_usize_when_bit_depth_two,
        BitDepth::Four => read_byte_for_usize_when_bit_depth_four,
        BitDepth::Eight => read_byte_for_usize_when_bit_depth_eight,
        BitDepth::Sixteen => read_byte_for_usize_when_bit_depth_sixteen,
    };

    let mut result = Vec::new();
    let mut tmp = [0; 8];
    let mut buf_iter = buf.iter();

    let channel_len = info.color_type.samples();
    let width = info.width as usize;
    let height = info.height as usize;
    let line_length = width * channel_len;

    let mut line = Vec::new();
    for _ in 0..height {
        line.clear();

        while let Some(tmp_size) = f(&mut buf_iter, &mut tmp)? {
            line.extend_from_slice(&tmp[..tmp_size]);

            if line.len() >= line_length {
                line.resize(line_length, 0);
                break;
            }
        }
        if line.len() < line_length {
            return Err(ResizedPngError::DecodingError);
        }

        result.extend_from_slice(&line);
    }

    Ok(result)
}

fn read_byte_for_usize_when_bit_depth_one(
    buf_iter: &mut Iter<u8>,
    output: &mut [usize; 8],
) -> Result<Option<usize>, ResizedPngError> {
    Ok(buf_iter.next().map(|t| {
        for (i, element) in output.iter_mut().enumerate().take(8) {
            *element = if (t << i) | 0b01111111 == u8::MAX {
                1
            } else {
                0
            };
        }
        8
    }))
}

fn read_byte_for_usize_when_bit_depth_two(
    buf_iter: &mut Iter<u8>,
    output: &mut [usize; 8],
) -> Result<Option<usize>, ResizedPngError> {
    Ok(buf_iter.next().map(|t| {
        for (i, element) in output.iter_mut().enumerate().take(4) {
            // VVxxxxxx -> 000000VV
            *element = (((t << (i * 2)) >> 6) & 0b00000011) as usize;
        }
        4
    }))
}

fn read_byte_for_usize_when_bit_depth_four(
    buf_iter: &mut Iter<u8>,
    output: &mut [usize; 8],
) -> Result<Option<usize>, ResizedPngError> {
    Ok(buf_iter.next().map(|t| {
        for (i, element) in output.iter_mut().enumerate().take(2) {
            // VVVVxxxx -> 0000VVVV
            *element = (((t << (i * 4)) >> 4) & 0b00001111) as usize;
        }
        2
    }))
}

fn read_byte_for_usize_when_bit_depth_eight(
    buf_iter: &mut Iter<u8>,
    output: &mut [usize; 8],
) -> Result<Option<usize>, ResizedPngError> {
    Ok(buf_iter.next().map(|t| {
        output[0] = *t as usize;
        1
    }))
}

fn read_byte_for_usize_when_bit_depth_sixteen(
    buf_iter: &mut Iter<u8>,
    output: &mut [usize; 8],
) -> Result<Option<usize>, ResizedPngError> {
    match (buf_iter.next(), buf_iter.next()) {
        (Some(t1), Some(t2)) => {
            // 1111111122222222
            output[0] = u16::from_be_bytes([*t1, *t2]) as usize;

            Ok(Some(1))
        }
        (Some(_), None) => Err(ResizedPngError::DecodingError),
        (None, _) => Ok(None),
    }
}

fn split_palette(raw: &[u8]) -> Result<Vec<[u8; 3]>, ResizedPngError> {
    let mut result = Vec::new();
    let palette_chunked = raw.chunks(3);

    for p in palette_chunked {
        if p.len() != 3 {
            return Err(ResizedPngError::DecodingError);
        }

        result.push([p[0], p[1], p[2]]);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod read_image_data {
        use super::*;

        #[test]
        fn success_when_valid_png_path() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.png");

            let (_data, width, height) = read_image_data(&path).unwrap();

            assert_eq!(width, 100);
            assert_eq!(height, 200);
        }

        #[test]
        fn failed_when_invalid_png_path() {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_target/image/sample.bmp");

            assert!(read_image_data(&path).is_err());
        }
    }

    mod write_png {
        use super::*;

        use tempfile::tempdir;

        #[test]
        fn success_when_valid_parameter() {
            let out_dir = tempdir().unwrap();

            let path = out_dir.path().join("test.png");
            let buf = [1, 2, 3, 4, 5, 6, 7, 8];
            let width = 2;
            let height = 1;

            write_png(&path, &buf, width, height).unwrap();

            assert!(path.exists());

            out_dir.close().unwrap();
        }
    }

    mod buf_to_rgba {
        use super::*;
        use std::borrow::Cow;

        #[test]
        fn success_when_valid_bytes_for_grayscale() {
            let buf = [0b10010000];
            let mut info = Info::with_size(2, 1);
            info.color_type = ColorType::Grayscale;
            info.bit_depth = BitDepth::One;

            assert_eq!(
                buf_to_rgba(&buf, &info).unwrap(),
                vec![u8::MAX, u8::MAX, u8::MAX, u8::MAX, 0, 0, 0, u8::MAX,]
            );
        }

        #[test]
        fn success_when_valid_bytes_for_grayscale_alpha() {
            let buf = [0b10010011, 0b01101100];
            let mut info = Info::with_size(2, 2);
            info.color_type = ColorType::GrayscaleAlpha;
            info.bit_depth = BitDepth::Two;

            assert_eq!(
                buf_to_rgba(&buf, &info).unwrap(),
                vec![
                    0b10000000, 0b10000000, 0b10000000, 0b01111111, 0, 0, 0, 0b11111111,
                    0b01111111, 0b01111111, 0b01111111, 0b10000000, 0b11111111, 0b11111111,
                    0b11111111, 0,
                ]
            );
        }

        #[test]
        fn success_when_valid_bytes_for_rgb() {
            let buf = [
                0b10010011, 0b01101100, 0b10010011, 0b01101100, 0b10010011, 0b01101100,
            ];
            let mut info = Info::with_size(2, 2);
            info.color_type = ColorType::Rgb;
            info.bit_depth = BitDepth::Four;

            assert_eq!(
                buf_to_rgba(&buf, &info).unwrap(),
                vec![
                    0b10011111,
                    0b00111111,
                    0b01100000,
                    u8::MAX,
                    0b11000000,
                    0b10011111,
                    0b00111111,
                    u8::MAX,
                    0b01100000,
                    0b11000000,
                    0b10011111,
                    u8::MAX,
                    0b00111111,
                    0b01100000,
                    0b11000000,
                    u8::MAX,
                ]
            );
        }

        #[test]
        fn success_when_valid_bytes_for_rgba() {
            let buf = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
            let mut info = Info::with_size(2, 2);
            info.color_type = ColorType::Rgba;
            info.bit_depth = BitDepth::Eight;

            assert_eq!(
                buf_to_rgba(&buf, &info).unwrap(),
                vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
            );
        }

        #[test]
        fn success_when_valid_bytes_for_indexed_and_valid_palette() {
            let buf = [0, 0, 0, 1, 0, 1, 0, 0];
            let mut info = Info::with_size(2, 2);
            info.color_type = ColorType::Indexed;
            info.bit_depth = BitDepth::Sixteen;

            let mut palette_raw: [u8; u16::MAX as usize * 3] = [0; u16::MAX as usize * 3];
            palette_raw[0] = 1;
            palette_raw[1] = 2;
            palette_raw[2] = 3;
            palette_raw[3] = 4;
            palette_raw[4] = 5;
            palette_raw[5] = 6;
            info.palette = Some(Cow::from(&palette_raw[..]));

            assert_eq!(
                buf_to_rgba(&buf, &info).unwrap(),
                vec![
                    1,
                    2,
                    3,
                    u8::MAX,
                    4,
                    5,
                    6,
                    u8::MAX,
                    4,
                    5,
                    6,
                    u8::MAX,
                    1,
                    2,
                    3,
                    u8::MAX
                ]
            );
        }

        #[test]
        fn failed_when_invalid_bytes_for_indexed_and_valid_palette() {
            let buf = [0, 0, 0, 1, 0, 1, 0];
            let mut info = Info::with_size(2, 2);
            info.color_type = ColorType::Indexed;
            info.bit_depth = BitDepth::Sixteen;

            let mut palette_raw: [u8; u16::MAX as usize * 3] = [0; u16::MAX as usize * 3];
            palette_raw[0] = 1;
            palette_raw[1] = 2;
            palette_raw[2] = 3;
            palette_raw[3] = 4;
            palette_raw[4] = 5;
            palette_raw[5] = 6;
            info.palette = Some(Cow::from(&palette_raw[..]));

            assert!(buf_to_rgba(&buf, &info).is_err());
        }

        #[test]
        fn failed_when_valid_bytes_for_indexed_and_invalid_pallete() {
            let buf = [0, 0, 0, 1, 0, 1, 0, 0];
            let mut info = Info::with_size(2, 2);
            info.color_type = ColorType::Indexed;
            info.bit_depth = BitDepth::Sixteen;

            let palette_raw: [u8; 1] = [0; 1];
            info.palette = Some(Cow::from(&palette_raw[..]));

            assert!(buf_to_rgba(&buf, &info).is_err());
        }

        #[test]
        fn failed_when_valid_bytes_for_indexed_and_no_palette() {
            let buf = [0, 0, 0, 1, 0, 1, 0, 0];
            let mut info = Info::with_size(2, 2);
            info.color_type = ColorType::Indexed;
            info.bit_depth = BitDepth::Sixteen;

            info.palette = None;

            assert!(buf_to_rgba(&buf, &info).is_err());
        }

        #[test]
        fn failed_when_result_is_too_short() {
            let buf = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
            let mut info = Info::with_size(2, 2);
            info.color_type = ColorType::Rgba;
            info.bit_depth = BitDepth::Eight;

            assert!(buf_to_rgba(&buf, &info).is_err());
        }
    }

    mod read_bytes_for_bit_depth_8 {
        use super::*;

        #[test]
        fn checking_value_when_bit_depth_one() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(2, 2);
            info.color_type = ColorType::Rgba;
            info.bit_depth = BitDepth::One;

            assert_eq!(
                read_bytes_for_bit_depth_8(&buf, &info).unwrap(),
                vec![
                    u8::MAX,
                    0,
                    0,
                    u8::MAX,
                    u8::MAX,
                    u8::MAX,
                    0,
                    0,
                    0,
                    u8::MAX,
                    u8::MAX,
                    0,
                    0,
                    0,
                    u8::MAX,
                    u8::MAX,
                ]
            );
        }

        #[test]
        fn checking_value_when_bit_depth_two() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(2, 1);
            info.color_type = ColorType::Rgb;
            info.bit_depth = BitDepth::Two;

            assert_eq!(
                read_bytes_for_bit_depth_8(&buf, &info).unwrap(),
                vec![0b10000000, 0b01111111, 0b11111111, 0b00000000, 0b01111111, 0b10000000,]
            );
        }

        #[test]
        fn checking_value_when_bit_depth_four() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(2, 1);
            info.color_type = ColorType::GrayscaleAlpha;
            info.bit_depth = BitDepth::Four;

            assert_eq!(
                read_bytes_for_bit_depth_8(&buf, &info).unwrap(),
                vec![0b10011111, 0b11000000, 0b01100000, 0b00111111]
            );
        }

        #[test]
        fn checking_value_when_bit_depth_eight() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(1, 2);
            info.color_type = ColorType::Grayscale;
            info.bit_depth = BitDepth::Eight;

            assert_eq!(
                read_bytes_for_bit_depth_8(&buf, &info).unwrap(),
                vec![0b10011100, 0b01100011]
            );
        }

        #[test]
        fn checking_value_when_bit_depth_sixteen() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(1, 1);
            info.color_type = ColorType::Indexed;
            info.bit_depth = BitDepth::Sixteen;

            assert_eq!(
                read_bytes_for_bit_depth_8(&buf, &info).unwrap(),
                vec![0b10011100]
            );
        }
    }

    mod read_bytes_for_usize {
        use super::*;

        #[test]
        fn success_when_bit_depth_one() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(2, 2);
            info.color_type = ColorType::Rgba;
            info.bit_depth = BitDepth::One;

            assert_eq!(
                read_bytes_for_usize(&buf, &info).unwrap(),
                vec![1, 0, 0, 1, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 1,]
            )
        }

        #[test]
        fn success_when_bit_depth_two() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(2, 1);
            info.color_type = ColorType::Rgb;
            info.bit_depth = BitDepth::Two;

            assert_eq!(
                read_bytes_for_usize(&buf, &info).unwrap(),
                vec![2, 1, 3, 0, 1, 2]
            )
        }

        #[test]
        fn success_when_bit_depth_four() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(2, 1);
            info.color_type = ColorType::GrayscaleAlpha;
            info.bit_depth = BitDepth::Four;

            assert_eq!(
                read_bytes_for_usize(&buf, &info).unwrap(),
                vec![0b01001, 0b1100, 0b0110, 0b0011]
            )
        }

        #[test]
        fn success_when_bit_depth_eight() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(1, 2);
            info.color_type = ColorType::Grayscale;
            info.bit_depth = BitDepth::Eight;

            assert_eq!(
                read_bytes_for_usize(&buf, &info).unwrap(),
                vec![0b10011100, 0b01100011]
            )
        }

        #[test]
        fn success_when_bit_depth_sixteen_and_valid_bytes() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(1, 1);
            info.color_type = ColorType::Indexed;
            info.bit_depth = BitDepth::Sixteen;

            assert_eq!(
                read_bytes_for_usize(&buf, &info).unwrap(),
                vec![0b1001110001100011]
            )
        }

        #[test]
        fn failed_when_bit_depth_sixteen_and_invalid_bytes() {
            let buf = [0b10011100, 0b01100011, 0b0011001111];
            let mut info = Info::with_size(2, 1);
            info.color_type = ColorType::Indexed;
            info.bit_depth = BitDepth::Sixteen;

            assert!(read_bytes_for_usize(&buf, &info).is_err())
        }
    }
}
