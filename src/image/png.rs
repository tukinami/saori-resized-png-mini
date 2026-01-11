use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::{borrow::Cow, fs::File};

use png::{BitDepth, ColorType, Decoder, Encoder, Info};

use crate::error::ResizedPngError;

use super::ImageData;

pub(crate) fn read_image_data(path: &PathBuf) -> Result<ImageData, ResizedPngError> {
    let fs = File::open(path)?;
    let reader = BufReader::new(fs);
    let decoder = Decoder::new(reader);
    let mut reader = decoder.read_info()?;

    let buf_size = reader
        .output_buffer_size()
        .ok_or(ResizedPngError::InputSizeError)?;
    let mut buf = vec![0; buf_size];
    let output_info = reader.next_frame(&mut buf)?;
    let bytes = &buf[..output_info.buffer_size()];

    let info = reader.info();

    let mut result = buf_to_rgba(bytes, info)?;
    let size = info.size();
    adjust_length(&mut result, (size.0 * size.1 * 4) as usize)?;

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

fn adjust_length(buf: &mut Vec<u8>, size: usize) -> Result<(), ResizedPngError> {
    if buf.len() < size {
        Err(ResizedPngError::DecodingError)
    } else {
        buf.resize(size, 0);
        Ok(())
    }
}

fn buf_to_rgba(buf: &[u8], info: &Info) -> Result<Vec<u8>, ResizedPngError> {
    let bytes = match info.color_type {
        ColorType::Indexed => {
            return buf_to_rgba_from_indexed(buf, &info.bit_depth, info.palette.as_ref())
        }
        _ => read_bytes_for_bit_depth_8(buf, &info.bit_depth),
    };

    match info.color_type {
        ColorType::Grayscale => Ok(bytes.iter().flat_map(|v| [*v, *v, *v, u8::MAX]).collect()),
        ColorType::GrayscaleAlpha => Ok(bytes
            .chunks_exact(2)
            .flat_map(|v| [v[0], v[0], v[0], v[1]])
            .collect()),
        ColorType::Rgb => Ok(bytes
            .chunks_exact(3)
            .flat_map(|v| [v[0], v[1], v[2], u8::MAX])
            .collect()),
        ColorType::Rgba => Ok(bytes
            .chunks_exact(4)
            .flat_map(|v| [v[0], v[1], v[2], v[3]])
            .collect()),
        ColorType::Indexed => unreachable!("early returned."),
    }
}

fn buf_to_rgba_from_indexed(
    buf: &[u8],
    bit_depth: &BitDepth,
    palette_raw: Option<&Cow<[u8]>>,
) -> Result<Vec<u8>, ResizedPngError> {
    let pallete = match palette_raw {
        Some(v) => split_palette(v)?,
        None => return Err(ResizedPngError::DecodingError),
    };
    let indices = read_bytes_for_usize(buf, bit_depth);

    indices
        .iter()
        .try_fold(Vec::new(), |mut acc, v| {
            pallete.get(*v).map(|p| {
                acc.push(p[0]);
                acc.push(p[1]);
                acc.push(p[2]);
                acc.push(u8::MAX);
                acc
            })
        })
        .ok_or(ResizedPngError::DecodingError)
}

fn read_bytes_for_bit_depth_8(buf: &[u8], bit_depth: &BitDepth) -> Vec<u8> {
    match bit_depth {
        BitDepth::One => buf
            .iter()
            .flat_map(read_byte_depth_1)
            .map(|v| bit_to_u8(v, 1))
            .collect(),
        BitDepth::Two => buf
            .iter()
            .flat_map(read_byte_depth_2)
            .map(|v| bit_to_u8(v, 2))
            .collect(),
        BitDepth::Four => buf
            .iter()
            .flat_map(read_byte_depth_4)
            .map(|v| bit_to_u8(v, 4))
            .collect(),
        BitDepth::Eight => buf.to_vec(),
        BitDepth::Sixteen => buf.chunks_exact(2).flat_map(|v| [v[0]]).collect(),
    }
}

fn read_bytes_for_usize(buf: &[u8], bit_depth: &BitDepth) -> Vec<usize> {
    match bit_depth {
        BitDepth::One => buf
            .iter()
            .flat_map(read_byte_depth_1)
            .map(|v| v as usize)
            .collect(),
        BitDepth::Two => buf
            .iter()
            .flat_map(read_byte_depth_2)
            .map(|v| v as usize)
            .collect(),
        BitDepth::Four => buf
            .iter()
            .flat_map(read_byte_depth_4)
            .map(|v| v as usize)
            .collect(),
        BitDepth::Eight => buf.iter().map(|v| *v as usize).collect(),
        BitDepth::Sixteen => buf
            .chunks_exact(2)
            .flat_map(|v| [(((v[0] as usize) << 8) | v[1] as usize)])
            .collect(),
    }
}

fn bit_to_u8(v: u8, bit: u32) -> u8 {
    let v = v << (8 - bit);
    if v.trailing_zeros() == (8 - bit) {
        v | (u8::MAX >> bit)
    } else {
        v
    }
}

fn read_byte_depth_1(v: &u8) -> [u8; 8] {
    [
        (v & (1 << 7)) >> 7,
        (v & (1 << 6)) >> 6,
        (v & (1 << 5)) >> 5,
        (v & (1 << 4)) >> 4,
        (v & (1 << 3)) >> 3,
        (v & (1 << 2)) >> 2,
        (v & (1 << 1)) >> 1,
        (v & (1 << 0)),
    ]
}

fn read_byte_depth_2(v: &u8) -> [u8; 4] {
    [
        (v & ((1 << 6) | (1 << 7))) >> 6,
        (v & ((1 << 4) | (1 << 5))) >> 4,
        (v & ((1 << 2) | (1 << 3))) >> 2,
        (v & ((1 << 0) | (1 << 1))),
    ]
}

fn read_byte_depth_4(v: &u8) -> [u8; 2] {
    [
        (v & ((1 << 4) | (1 << 5) | (1 << 6) | (1 << 7))) >> 4,
        (v & ((1 << 0) | (1 << 1) | (1 << 2) | (1 << 3))),
    ]
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
                [
                    [u8::MAX, u8::MAX, u8::MAX, u8::MAX],
                    [0, 0, 0, u8::MAX],
                    [0, 0, 0, u8::MAX],
                    [u8::MAX, u8::MAX, u8::MAX, u8::MAX],
                    [0, 0, 0, u8::MAX],
                    [0, 0, 0, u8::MAX],
                    [0, 0, 0, u8::MAX],
                    [0, 0, 0, u8::MAX],
                ]
                .iter()
                .flat_map(|v| v.iter().cloned())
                .collect::<Vec<u8>>()
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

        // #[test]
        // fn failed_when_result_is_too_short() {
        //     let buf = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        //     let mut info = Info::with_size(2, 2);
        //     info.color_type = ColorType::Rgba;
        //     info.bit_depth = BitDepth::Eight;

        //     assert!(buf_to_rgba(&buf, &info).is_err());
        // }
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
                read_bytes_for_bit_depth_8(&buf, &info.bit_depth),
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
                read_bytes_for_bit_depth_8(&buf, &info.bit_depth),
                vec![
                    0b10000000, 0b01111111, 0b11111111, 0b00000000, 0b01111111, 0b10000000,
                    0b00000000, 0b11111111
                ]
            );
        }

        #[test]
        fn checking_value_when_bit_depth_four() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(2, 1);
            info.color_type = ColorType::GrayscaleAlpha;
            info.bit_depth = BitDepth::Four;

            assert_eq!(
                read_bytes_for_bit_depth_8(&buf, &info.bit_depth),
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
                read_bytes_for_bit_depth_8(&buf, &info.bit_depth),
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
                read_bytes_for_bit_depth_8(&buf, &info.bit_depth),
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
                read_bytes_for_usize(&buf, &info.bit_depth),
                vec![1, 0, 0, 1, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 1,]
            );
        }

        #[test]
        fn success_when_bit_depth_two() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(2, 1);
            info.color_type = ColorType::Rgb;
            info.bit_depth = BitDepth::Two;

            assert_eq!(
                read_bytes_for_usize(&buf, &info.bit_depth),
                vec![2, 1, 3, 0, 1, 2, 0, 3]
            );
        }

        #[test]
        fn success_when_bit_depth_four() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(2, 1);
            info.color_type = ColorType::GrayscaleAlpha;
            info.bit_depth = BitDepth::Four;

            assert_eq!(
                read_bytes_for_usize(&buf, &info.bit_depth),
                vec![0b01001, 0b1100, 0b0110, 0b0011]
            );
        }

        #[test]
        fn success_when_bit_depth_eight() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(1, 2);
            info.color_type = ColorType::Grayscale;
            info.bit_depth = BitDepth::Eight;

            assert_eq!(
                read_bytes_for_usize(&buf, &info.bit_depth),
                vec![0b10011100, 0b01100011]
            );
        }

        #[test]
        fn success_when_bit_depth_sixteen_and_valid_bytes() {
            let buf = [0b10011100, 0b01100011];
            let mut info = Info::with_size(1, 1);
            info.color_type = ColorType::Indexed;
            info.bit_depth = BitDepth::Sixteen;

            assert_eq!(
                read_bytes_for_usize(&buf, &info.bit_depth),
                vec![0b1001110001100011]
            );
        }

        // #[test]
        // fn failed_when_bit_depth_sixteen_and_invalid_bytes() {
        //     let buf = [0b10011100, 0b01100011, 0b0011001111];
        //     let mut info = Info::with_size(2, 1);
        //     info.color_type = ColorType::Indexed;
        //     info.bit_depth = BitDepth::Sixteen;

        //     assert!(read_bytes_for_usize(&buf, &info).is_err())
        // }
    }
}
