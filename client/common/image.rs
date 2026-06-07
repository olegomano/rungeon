extern crate handle;

use std::fmt;

/// Describes how an embedded or external blob should be interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSourceFormat {
    /// Blob is already `width * height * 4` bytes in ARGB8888 byte order.
    RawArgb {
        width: u32,
        height: u32,
    },
    /// Blob is an encoded PNG file.
    Png,
}

/// Metadata for a decoded CPU-resident image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageMeta {
    pub width: u32,
    pub height: u32,
}

impl ImageMeta {
    pub fn PixelCount(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    pub fn ByteLen(&self) -> usize {
        self.PixelCount() * 4
    }
}

#[derive(Debug)]
pub enum ImageError {
    SizeMismatch {
        expected: usize,
        actual: usize,
    },
    DecodeFailed(String),
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::SizeMismatch { expected, actual } => {
                write!(
                    f,
                    "image blob size mismatch: expected {} bytes, got {}",
                    expected, actual
                )
            }
            ImageError::DecodeFailed(msg) => write!(f, "failed to decode image: {}", msg),
        }
    }
}

#[derive(Debug)]
enum ImageStorage {
    Owned(Box<[u8]>),
    Static(&'static [u8]),
}

/// CPU-resident image stored as ARGB8888 pixels (alpha, red, green, blue).
///
/// Instances are intended to live in a sparse buffer and be referenced through
/// [`handle::handle_t<Image>`].
#[derive(Debug)]
pub struct Image {
    storage: ImageStorage,
    meta: ImageMeta,
}

impl Image {
    pub fn Meta(&self) -> ImageMeta {
        self.meta
    }

    pub fn Data(&self) -> *const u8 {
        self.AsSlice().as_ptr()
    }

    pub fn Len(&self) -> usize {
        self.AsSlice().len()
    }

    pub fn AsSlice(&self) -> &[u8] {
        match &self.storage {
            ImageStorage::Owned(bytes) => bytes,
            ImageStorage::Static(bytes) => bytes,
        }
    }

    /// Build an image from a binary blob and source-format metadata.
    pub fn FromBlob(blob: &[u8], source: ImageSourceFormat) -> Result<Self, ImageError> {
        match source {
            ImageSourceFormat::RawArgb { width, height } => Self::FromRawArgbBlob(blob, width, height),
            ImageSourceFormat::Png => Self::FromPngBlob(blob),
        }
    }

    /// Build an image from a compile-time embedded asset blob.
    pub fn FromEmbeddedAsset(
        blob: &'static [u8],
        source: ImageSourceFormat,
    ) -> Result<Self, ImageError> {
        match source {
            ImageSourceFormat::RawArgb { width, height } => {
                Self::FromRawArgbEmbedded(blob, width, height)
            }
            ImageSourceFormat::Png => Self::FromPngBlob(blob),
        }
    }

    fn FromRawArgbBlob(blob: &[u8], width: u32, height: u32) -> Result<Self, ImageError> {
        let meta = ImageMeta { width, height };
        let expected = meta.ByteLen();
        if blob.len() != expected {
            return Err(ImageError::SizeMismatch {
                expected,
                actual: blob.len(),
            });
        }

        Ok(Self {
            storage: ImageStorage::Owned(blob.to_vec().into_boxed_slice()),
            meta,
        })
    }

    fn FromRawArgbEmbedded(
        blob: &'static [u8],
        width: u32,
        height: u32,
    ) -> Result<Self, ImageError> {
        let meta = ImageMeta { width, height };
        let expected = meta.ByteLen();
        if blob.len() != expected {
            return Err(ImageError::SizeMismatch {
                expected,
                actual: blob.len(),
            });
        }

        Ok(Self {
            storage: ImageStorage::Static(blob),
            meta,
        })
    }

    fn FromPngBlob(blob: &[u8]) -> Result<Self, ImageError> {
        let decoded = image::load_from_memory(blob)
            .map_err(|err| ImageError::DecodeFailed(err.to_string()))?;

        let rgba = decoded.to_rgba8();
        let (width, height) = rgba.dimensions();
        let meta = ImageMeta { width, height };

        let mut argb = Vec::with_capacity(meta.ByteLen());
        for pixel in rgba.pixels() {
            argb.push(pixel[3]);
            argb.push(pixel[0]);
            argb.push(pixel[1]);
            argb.push(pixel[2]);
        }

        Ok(Self {
            storage: ImageStorage::Owned(argb.into_boxed_slice()),
            meta,
        })
    }
}
