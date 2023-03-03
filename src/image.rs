pub(crate) mod bmp;
pub(crate) mod gif;
pub(crate) mod jpeg;
pub(crate) mod png;
pub(crate) mod webp;

pub(crate) type ImageData = (Vec<u8>, u32, u32);
