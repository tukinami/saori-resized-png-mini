#[derive(Debug)]
pub(crate) enum ResizedPngError {
    Unsupported,
    NotFound,
    IoError,
    DecodingError,
    EncodingError,
    ParameterError,
    LimitsError,
    InputSizeError,
}

impl ResizedPngError {
    pub(crate) fn to_code(&self) -> u32 {
        match self {
            Self::Unsupported => 1,
            Self::NotFound => 2,
            Self::IoError => 3,
            Self::DecodingError => 4,
            Self::EncodingError => 5,
            Self::ParameterError => 6,
            Self::LimitsError => 7,
            Self::InputSizeError => 8,
        }
    }
}

impl From<std::io::Error> for ResizedPngError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => Self::NotFound,
            _ => Self::IoError,
        }
    }
}

impl From<tinybmp::ParseError> for ResizedPngError {
    fn from(e: tinybmp::ParseError) -> Self {
        match e {
            tinybmp::ParseError::InvalidImageDimensions => Self::ParameterError,
            _ => Self::Unsupported,
        }
    }
}

impl From<gif::DecodingError> for ResizedPngError {
    fn from(e: gif::DecodingError) -> Self {
        match e {
            gif::DecodingError::Format(_) => Self::DecodingError,
            gif::DecodingError::Io(e) => e.into(),
        }
    }
}

impl From<jpeg_decoder::Error> for ResizedPngError {
    fn from(e: jpeg_decoder::Error) -> Self {
        match e {
            jpeg_decoder::Error::Format(_) => Self::DecodingError,
            jpeg_decoder::Error::Unsupported(_) => Self::Unsupported,
            jpeg_decoder::Error::Io(e) => e.into(),
            jpeg_decoder::Error::Internal(_) => Self::DecodingError,
        }
    }
}

impl From<png::DecodingError> for ResizedPngError {
    fn from(e: png::DecodingError) -> Self {
        match e {
            png::DecodingError::IoError(e) => e.into(),
            png::DecodingError::Format(_) => Self::DecodingError,
            png::DecodingError::Parameter(_) => Self::ParameterError,
            png::DecodingError::LimitsExceeded => Self::LimitsError,
        }
    }
}

impl From<png::EncodingError> for ResizedPngError {
    fn from(e: png::EncodingError) -> Self {
        match e {
            png::EncodingError::IoError(e) => e.into(),
            png::EncodingError::Format(_) => Self::EncodingError,
            png::EncodingError::Parameter(_) => Self::ParameterError,
            png::EncodingError::LimitsExceeded => Self::LimitsError,
        }
    }
}

impl From<resize::Error> for ResizedPngError {
    fn from(e: resize::Error) -> Self {
        match e {
            resize::Error::InvalidParameters => Self::ParameterError,
            resize::Error::OutOfMemory => Self::LimitsError,
        }
    }
}
