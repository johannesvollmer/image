use std::io;
use std::io::Write;
use std::path::Path;
use std::u32;

#[cfg(feature = "gif")]
use crate::codecs::gif;
#[cfg(feature = "png")]
use crate::codecs::png;
#[cfg(feature = "pnm")]
use crate::codecs::pnm;

use crate::buffer_::{
    BgrImage, BgraImage, ConvertBuffer, GrayAlphaImage, GrayAlpha16Image,
    GrayImage, Gray16Image, ImageBuffer, RgbImage, Rgb16Image, RgbaImage,
    Rgba16Image,
};
use crate::color::{self, IntoColor};
use crate::error::{ImageError, ImageFormatHint, ImageResult, ParameterError, ParameterErrorKind, UnsupportedError, UnsupportedErrorKind};
use crate::flat::FlatSamples;
use crate::{image, Rgba32FImage, Rgb32FImage};
use crate::image::{GenericImage, GenericImageView, ImageDecoder, ImageFormat, ImageOutputFormat};
use crate::io::free_functions;
use crate::imageops;
use crate::math::resize_dimensions;
use crate::traits::Pixel;

/// A Dynamic Image
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum DynamicImage {
    /// Each pixel in this image is 8-bit Luma
    ImageLuma8(GrayImage),

    /// Each pixel in this image is 8-bit Luma with alpha
    ImageLumaA8(GrayAlphaImage),

    /// Each pixel in this image is 8-bit Rgb
    ImageRgb8(RgbImage),

    /// Each pixel in this image is 8-bit Rgb with alpha
    ImageRgba8(RgbaImage),

    /// Each pixel in this image is 8-bit Bgr
    ImageBgr8(BgrImage),

    /// Each pixel in this image is 8-bit Bgr with alpha
    ImageBgra8(BgraImage),

    /// Each pixel in this image is 16-bit Luma
    ImageLuma16(Gray16Image),

    /// Each pixel in this image is 16-bit Luma with alpha
    ImageLumaA16(GrayAlpha16Image),

    /// Each pixel in this image is 16-bit Rgb
    ImageRgb16(Rgb16Image),

    /// Each pixel in this image is 16-bit Rgb with alpha
    ImageRgba16(Rgba16Image),

    /// Each pixel in this image is 32-bit float Rgb
    ImageRgb32F(Rgb32FImage),

    /// Each pixel in this image is 32-bit float Rgb with alpha
    ImageRgba32F(Rgba32FImage),
}

macro_rules! dynamic_map(
        ($dynimage: expr, ref $image: ident => $action: expr) => (
                match $dynimage {
                        DynamicImage::ImageLuma8(ref $image) => DynamicImage::ImageLuma8($action),
                        DynamicImage::ImageLumaA8(ref $image) => DynamicImage::ImageLumaA8($action),
                        DynamicImage::ImageRgb8(ref $image) => DynamicImage::ImageRgb8($action),
                        DynamicImage::ImageRgba8(ref $image) => DynamicImage::ImageRgba8($action),
                        DynamicImage::ImageBgr8(ref $image) => DynamicImage::ImageBgr8($action),
                        DynamicImage::ImageBgra8(ref $image) => DynamicImage::ImageBgra8($action),
                        DynamicImage::ImageLuma16(ref $image) => DynamicImage::ImageLuma16($action),
                        DynamicImage::ImageLumaA16(ref $image) => DynamicImage::ImageLumaA16($action),
                        DynamicImage::ImageRgb16(ref $image) => DynamicImage::ImageRgb16($action),
                        DynamicImage::ImageRgba16(ref $image) => DynamicImage::ImageRgba16($action),
                        DynamicImage::ImageRgb32F(ref $image) => DynamicImage::ImageRgb32F($action),
                        DynamicImage::ImageRgba32F(ref $image) => DynamicImage::ImageRgba32F($action),
                }
        );

        ($dynimage: expr, ref mut $image: ident => $action: expr) => (
                match $dynimage {
                        DynamicImage::ImageLuma8(ref mut $image) => DynamicImage::ImageLuma8($action),
                        DynamicImage::ImageLumaA8(ref mut $image) => DynamicImage::ImageLumaA8($action),
                        DynamicImage::ImageRgb8(ref mut $image) => DynamicImage::ImageRgb8($action),
                        DynamicImage::ImageRgba8(ref mut $image) => DynamicImage::ImageRgba8($action),
                        DynamicImage::ImageBgr8(ref mut $image) => DynamicImage::ImageBgr8($action),
                        DynamicImage::ImageBgra8(ref mut $image) => DynamicImage::ImageBgra8($action),
                        DynamicImage::ImageLuma16(ref mut $image) => DynamicImage::ImageLuma16($action),
                        DynamicImage::ImageLumaA16(ref mut $image) => DynamicImage::ImageLumaA16($action),
                        DynamicImage::ImageRgb16(ref mut $image) => DynamicImage::ImageRgb16($action),
                        DynamicImage::ImageRgba16(ref mut $image) => DynamicImage::ImageRgba16($action),
                        DynamicImage::ImageRgb32F(ref mut $image) => DynamicImage::ImageRgb32F($action),
                        DynamicImage::ImageRgba32F(ref mut $image) => DynamicImage::ImageRgba32F($action),
                }
        );

        ($dynimage: expr, ref $image: ident -> $action: expr) => (
                match $dynimage {
                        DynamicImage::ImageLuma8(ref $image) => $action,
                        DynamicImage::ImageLumaA8(ref $image) => $action,
                        DynamicImage::ImageRgb8(ref $image) => $action,
                        DynamicImage::ImageRgba8(ref $image) => $action,
                        DynamicImage::ImageBgr8(ref $image) => $action,
                        DynamicImage::ImageBgra8(ref $image) => $action,
                        DynamicImage::ImageLuma16(ref $image) => $action,
                        DynamicImage::ImageLumaA16(ref $image) => $action,
                        DynamicImage::ImageRgb16(ref $image) => $action,
                        DynamicImage::ImageRgba16(ref $image) => $action,
                        DynamicImage::ImageRgb32F(ref $image) => $action,
                        DynamicImage::ImageRgba32F(ref $image) => $action,
                }
        );

        ($dynimage: expr, ref mut $image: ident -> $action: expr) => (
                match $dynimage {
                        DynamicImage::ImageLuma8(ref mut $image) => $action,
                        DynamicImage::ImageLumaA8(ref mut $image) => $action,
                        DynamicImage::ImageRgb8(ref mut $image) => $action,
                        DynamicImage::ImageRgba8(ref mut $image) => $action,
                        DynamicImage::ImageBgr8(ref mut $image) => $action,
                        DynamicImage::ImageBgra8(ref mut $image) => $action,
                        DynamicImage::ImageLuma16(ref mut $image) => $action,
                        DynamicImage::ImageLumaA16(ref mut $image) => $action,
                        DynamicImage::ImageRgb32F(ref mut $image) => $action,
                        DynamicImage::ImageRgba32F(ref mut $image) => $action,
                }
        );
);

impl DynamicImage {
    /// Creates a dynamic image backed by a buffer of grey pixels.
    pub fn new_luma8(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageLuma8(ImageBuffer::new(w, h))
    }

    /// Creates a dynamic image backed by a buffer of grey
    /// pixels with transparency.
    pub fn new_luma_a8(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageLumaA8(ImageBuffer::new(w, h))
    }

    /// Creates a dynamic image backed by a buffer of RGB pixels.
    pub fn new_rgb8(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(ImageBuffer::new(w, h))
    }

    /// Creates a dynamic image backed by a buffer of RGBA pixels.
    pub fn new_rgba8(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(ImageBuffer::new(w, h))
    }

    /// Creates a dynamic image backed by a buffer of BGRA pixels.
    pub fn new_bgra8(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageBgra8(ImageBuffer::new(w, h))
    }

    /// Creates a dynamic image backed by a buffer of BGR pixels.
    pub fn new_bgr8(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageBgr8(ImageBuffer::new(w, h))
    }

    /// Creates a dynamic image backed by a buffer of grey pixels.
    pub fn new_luma16(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageLuma16(ImageBuffer::new(w, h))
    }

    /// Creates a dynamic image backed by a buffer of grey
    /// pixels with transparency.
    pub fn new_luma_a16(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageLumaA16(ImageBuffer::new(w, h))
    }

    /// Creates a dynamic image backed by a buffer of RGB pixels.
    pub fn new_rgb16(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb16(ImageBuffer::new(w, h))
    }

    /// Creates a dynamic image backed by a buffer of RGBA pixels.
    pub fn new_rgba16(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgba16(ImageBuffer::new(w, h))
    }

    /// Creates a dynamic image backed by a buffer of RGB pixels.
    pub fn new_rgb32f(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb32F(ImageBuffer::new(w, h))
    }

    /// Creates a dynamic image backed by a buffer of RGBA pixels.
    pub fn new_rgba32f(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgba32F(ImageBuffer::new(w, h))
    }

    /// Decodes an encoded image into a dynamic image.
    pub fn from_decoder<'a>(decoder: impl ImageDecoder<'a>)
        -> ImageResult<Self>
    {
        decoder_to_image(decoder)
    }

    /// Returns a copy of this image as an RGB image.
    #[deprecated = "replaced by `to_rgb8`"]
    pub fn to_rgb(&self) -> RgbImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as an RGB image.
    pub fn to_rgb8(&self) -> RgbImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as an RGB image.
    pub fn to_rgb16(&self) -> Rgb16Image {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as an RGB image.
    pub fn to_rgb32f(&self) -> Rgb32FImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as an RGBA image.
    #[deprecated = "replaced by `to_rgba8`"]
    pub fn to_rgba(&self) -> RgbaImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as an RGBA image.
    pub fn to_rgba8(&self) -> RgbaImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as an RGBA image.
    pub fn to_rgba16(&self) -> Rgba16Image {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as an RGBA image.
    pub fn to_rgba32f(&self) -> Rgba32FImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as an BGR image.
    #[deprecated = "replaced by `to_bgr8`"]
    pub fn to_bgr(&self) -> BgrImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as an BGR image.
    pub fn to_bgr8(&self) -> BgrImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as an BGRA image.
    #[deprecated = "replaced by `to_bgra8`"]
    pub fn to_bgra(&self) -> BgraImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as an BGRA image.
    pub fn to_bgra8(&self) -> BgraImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as a Luma image.
    #[deprecated = "replaced by `to_luma8`"]
    pub fn to_luma(&self) -> GrayImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as a Luma image.
    pub fn to_luma8(&self) -> GrayImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as a Luma image.
    pub fn to_luma16(&self) -> Gray16Image {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as a LumaA image.
    #[deprecated = "replaced by `to_luma_alpha8`"]
    pub fn to_luma_alpha(&self) -> GrayAlphaImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as a LumaA image.
    pub fn to_luma_alpha8(&self) -> GrayAlphaImage {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Returns a copy of this image as a LumaA image.
    pub fn to_luma_alpha16(&self) -> GrayAlpha16Image {
        dynamic_map!(*self, ref p -> {
            p.convert()
        })
    }

    /// Consume the image and returns a RGB image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    #[deprecated = "replaced by `into_rgb8`"]
    pub fn into_rgb(self) -> RgbImage {
        match self {
            DynamicImage::ImageRgb8(x) => x,
            x => x.to_rgb8(),
        }
    }

    /// Consume the image and returns a RGB image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    pub fn into_rgb8(self) -> RgbImage {
        match self {
            DynamicImage::ImageRgb8(x) => x,
            x => x.to_rgb8(),
        }
    }

    /// Consume the image and returns a RGB image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    pub fn into_rgb16(self) -> Rgb16Image {
        match self {
            DynamicImage::ImageRgb16(x) => x,
            x => x.to_rgb16(),
        }
    }

    /// Consume the image and returns a RGB image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    pub fn into_rgb32f(self) -> Rgb32FImage {
        match self {
            DynamicImage::ImageRgb32F(x) => x,
            x => x.to_rgb32f(),
        }
    }

    /// Consume the image and returns a RGBA image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    #[deprecated = "replaced by `into_rgba8`"]
    pub fn into_rgba(self) -> RgbaImage {
        match self {
            DynamicImage::ImageRgba8(x) => x,
            x => x.to_rgba8(),
        }
    }

    /// Consume the image and returns a RGBA image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    pub fn into_rgba8(self) -> RgbaImage {
        match self {
            DynamicImage::ImageRgba8(x) => x,
            x => x.to_rgba8(),
        }
    }

    /// Consume the image and returns a RGBA image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    pub fn into_rgba16(self) -> Rgba16Image {
        match self {
            DynamicImage::ImageRgba16(x) => x,
            x => x.to_rgba16(),
        }
    }

    /// Consume the image and returns a RGBA image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    pub fn into_rgba32f(self) -> Rgba32FImage {
        match self {
            DynamicImage::ImageRgba32F(x) => x,
            x => x.to_rgba32f(),
        }
    }

    /// Consume the image and returns a BGR image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    #[deprecated = "replaced by `into_bgra8`"]
    pub fn into_bgr(self) -> BgrImage {
        match self {
            DynamicImage::ImageBgr8(x) => x,
            x => x.to_bgr8(),
        }
    }

    /// Consume the image and returns a BGR image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    pub fn into_bgr8(self) -> BgrImage {
        match self {
            DynamicImage::ImageBgr8(x) => x,
            x => x.to_bgr8(),
        }
    }

    // NOTE: DynamicImage::ImageBgr16 variant does not currently exist, so there is no `into_bgr16`

    /// Consume the image and returns a BGRA image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    #[deprecated = "replaced by `into_bgra8`"]
    pub fn into_bgra(self) -> BgraImage {
        match self {
            DynamicImage::ImageBgra8(x) => x,
            x => x.to_bgra8(),
        }
    }

    /// Consume the image and returns a BGRA image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    pub fn into_bgra8(self) -> BgraImage {
        match self {
            DynamicImage::ImageBgra8(x) => x,
            x => x.to_bgra8(),
        }
    }

    // NOTE: DynamicImage::ImageBgra16 variant does not currently exist, so there is no `into_bgra16`

    /// Consume the image and returns a Luma image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    #[deprecated = "replaced by `into_luma8`"]
    pub fn into_luma(self) -> GrayImage {
        match self {
            DynamicImage::ImageLuma8(x) => x,
            x => x.to_luma8(),
        }
    }

    /// Consume the image and returns a Luma image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    pub fn into_luma8(self) -> GrayImage {
        match self {
            DynamicImage::ImageLuma8(x) => x,
            x => x.to_luma8(),
        }
    }

    /// Consume the image and returns a Luma image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    pub fn into_luma16(self) -> Gray16Image {
        match self {
            DynamicImage::ImageLuma16(x) => x,
            x => x.to_luma16(),
        }
    }

    /// Consume the image and returns a LumaA image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    #[deprecated = "replaced by `into_luma_alpha8`"]
    pub fn into_luma_alpha(self) -> GrayAlphaImage {
        match self {
            DynamicImage::ImageLumaA8(x) => x,
            x => x.to_luma_alpha8(),
        }
    }

    /// Consume the image and returns a LumaA image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    pub fn into_luma_alpha8(self) -> GrayAlphaImage {
        match self {
            DynamicImage::ImageLumaA8(x) => x,
            x => x.to_luma_alpha8(),
        }
    }

    /// Consume the image and returns a LumaA image.
    ///
    /// If the image was already the correct format, it is returned as is.
    /// Otherwise, a copy is created.
    pub fn into_luma_alpha16(self) -> GrayAlpha16Image {
        match self {
            DynamicImage::ImageLumaA16(x) => x,
            x => x.to_luma_alpha16(),
        }
    }

    /// Return a cut-out of this image delimited by the bounding rectangle.
    ///
    /// Note: this method does *not* modify the object,
    /// and its signature will be replaced with `crop_imm()`'s in the 0.24 release
    pub fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> DynamicImage {
        dynamic_map!(*self, ref mut p => imageops::crop(p, x, y, width, height).to_image())
    }

    /// Return a cut-out of this image delimited by the bounding rectangle.
    pub fn crop_imm(&self, x: u32, y: u32, width: u32, height: u32) -> DynamicImage {
        dynamic_map!(*self, ref p => imageops::crop_imm(p, x, y, width, height).to_image())
    }

    /// Return a reference to an 8bit RGB image
    pub fn as_rgb8(&self) -> Option<&RgbImage> {
        match *self {
            DynamicImage::ImageRgb8(ref p) => Some(p),
            _ => None,
        }
    }

    /// Return a mutable reference to an 8bit RGB image
    pub fn as_mut_rgb8(&mut self) -> Option<&mut RgbImage> {
        match *self {
            DynamicImage::ImageRgb8(ref mut p) => Some(p),
            _ => None,
        }
    }

    /// Return a reference to an 8bit BGR image
    pub fn as_bgr8(&self) -> Option<&BgrImage> {
        match *self {
            DynamicImage::ImageBgr8(ref p) => Some(p),
            _ => None,
        }
    }

    /// Return a mutable reference to an 8bit BGR image
    pub fn as_mut_bgr8(&mut self) -> Option<&mut BgrImage> {
        match *self {
            DynamicImage::ImageBgr8(ref mut p) => Some(p),
            _ => None,
        }
    }

    /// Return a reference to an 8bit RGBA image
    pub fn as_rgba8(&self) -> Option<&RgbaImage> {
        match *self {
            DynamicImage::ImageRgba8(ref p) => Some(p),
            _ => None,
        }
    }

    /// Return a mutable reference to an 8bit RGBA image
    pub fn as_mut_rgba8(&mut self) -> Option<&mut RgbaImage> {
        match *self {
            DynamicImage::ImageRgba8(ref mut p) => Some(p),
            _ => None,
        }
    }

    /// Return a reference to an 8bit BGRA image
    pub fn as_bgra8(&self) -> Option<&BgraImage> {
        match *self {
            DynamicImage::ImageBgra8(ref p) => Some(p),
            _ => None,
        }
    }

    /// Return a mutable reference to an 8bit RGBA image
    pub fn as_mut_bgra8(&mut self) -> Option<&mut BgraImage> {
        match *self {
            DynamicImage::ImageBgra8(ref mut p) => Some(p),
            _ => None,
        }
    }

    /// Return a reference to an 8bit Grayscale image
    pub fn as_luma8(&self) -> Option<&GrayImage> {
        match *self {
            DynamicImage::ImageLuma8(ref p) => Some(p),
            _ => None,
        }
    }

    /// Return a mutable reference to an 8bit Grayscale image
    pub fn as_mut_luma8(&mut self) -> Option<&mut GrayImage> {
        match *self {
            DynamicImage::ImageLuma8(ref mut p) => Some(p),
            _ => None,
        }
    }

    /// Return a reference to an 8bit Grayscale image with an alpha channel
    pub fn as_luma_alpha8(&self) -> Option<&GrayAlphaImage> {
        match *self {
            DynamicImage::ImageLumaA8(ref p) => Some(p),
            _ => None,
        }
    }

    /// Return a mutable reference to an 8bit Grayscale image with an alpha channel
    pub fn as_mut_luma_alpha8(&mut self) -> Option<&mut GrayAlphaImage> {
        match *self {
            DynamicImage::ImageLumaA8(ref mut p) => Some(p),
            _ => None,
        }
    }

    /// Return a reference to an 16bit RGB image
    pub fn as_rgb16(&self) -> Option<&Rgb16Image> {
        match *self {
            DynamicImage::ImageRgb16(ref p) => Some(p),
            _ => None,
        }
    }

    /// Return a mutable reference to an 16bit RGB image
    pub fn as_mut_rgb16(&mut self) -> Option<&mut Rgb16Image> {
        match *self {
            DynamicImage::ImageRgb16(ref mut p) => Some(p),
            _ => None,
        }
    }

    /// Return a reference to an 32bit float RGB image
    pub fn as_rgb32f(&self) -> Option<&Rgb32FImage> {
        match *self {
            DynamicImage::ImageRgb32F(ref p) => Some(p),
            _ => None,
        }
    }

    /// Return a mutable reference to an 16bit float RGB image
    pub fn as_mut_rgb32f(&mut self) -> Option<&mut Rgb32FImage> {
        match *self {
            DynamicImage::ImageRgb32F(ref mut p) => Some(p),
            _ => None,
        }
    }

    /// Return a reference to an 16bit RGBA image
    pub fn as_rgba16(&self) -> Option<&Rgba16Image> {
        match *self {
            DynamicImage::ImageRgba16(ref p) => Some(p),
            _ => None,
        }
    }

    /// Return a mutable reference to an 16bit RGBA image
    pub fn as_mut_rgba16(&mut self) -> Option<&mut Rgba16Image> {
        match *self {
            DynamicImage::ImageRgba16(ref mut p) => Some(p),
            _ => None,
        }
    }

    /// Return a reference to an 16bit RGBA image
    pub fn as_rgba32f(&self) -> Option<&Rgba32FImage> {
        match *self {
            DynamicImage::ImageRgba32F(ref p) => Some(p),
            _ => None,
        }
    }

    /// Return a mutable reference to an 16bit RGBA image
    pub fn as_mut_rgba32f(&mut self) -> Option<&mut Rgba32FImage> {
        match *self {
            DynamicImage::ImageRgba32F(ref mut p) => Some(p),
            _ => None,
        }
    }

    /// Return a reference to an 16bit Grayscale image
    pub fn as_luma16(&self) -> Option<&Gray16Image> {
        match *self {
            DynamicImage::ImageLuma16(ref p) => Some(p),
            _ => None,
        }
    }

    /// Return a mutable reference to an 16bit Grayscale image
    pub fn as_mut_luma16(&mut self) -> Option<&mut Gray16Image> {
        match *self {
            DynamicImage::ImageLuma16(ref mut p) => Some(p),
            _ => None,
        }
    }

    /// Return a reference to an 16bit Grayscale image with an alpha channel
    pub fn as_luma_alpha16(&self) -> Option<&GrayAlpha16Image> {
        match *self {
            DynamicImage::ImageLumaA16(ref p) => Some(p),
            _ => None,
        }
    }

    /// Return a mutable reference to an 16bit Grayscale image with an alpha channel
    pub fn as_mut_luma_alpha16(&mut self) -> Option<&mut GrayAlpha16Image> {
        match *self {
            DynamicImage::ImageLumaA16(ref mut p) => Some(p),
            _ => None,
        }
    }

    /// Return a view on the raw sample buffer for 8 bit per channel images.
    pub fn as_flat_samples_u8(&self) -> Option<FlatSamples<&[u8]>> {
        match *self {
            DynamicImage::ImageLuma8(ref p) => Some(p.as_flat_samples()),
            DynamicImage::ImageLumaA8(ref p) => Some(p.as_flat_samples()),
            DynamicImage::ImageRgb8(ref p) => Some(p.as_flat_samples()),
            DynamicImage::ImageRgba8(ref p) => Some(p.as_flat_samples()),
            DynamicImage::ImageBgr8(ref p) => Some(p.as_flat_samples()),
            DynamicImage::ImageBgra8(ref p) => Some(p.as_flat_samples()),
            _ => None,
        }
    }

    /// Return a view on the raw sample buffer for 16 bit per channel images.
    pub fn as_flat_samples_u16(&self) -> Option<FlatSamples<&[u16]>> {
        match *self {
            DynamicImage::ImageLuma16(ref p) => Some(p.as_flat_samples()),
            DynamicImage::ImageLumaA16(ref p) => Some(p.as_flat_samples()),
            DynamicImage::ImageRgb16(ref p) => Some(p.as_flat_samples()),
            DynamicImage::ImageRgba16(ref p) => Some(p.as_flat_samples()),
            _ => None,
        }
    }

    /// Return a view on the raw sample buffer for 32 bit per channel images.
    pub fn as_flat_samples_f32(&self) -> Option<FlatSamples<&[f32]>> {
        match *self {
            DynamicImage::ImageRgb32F(ref p) => Some(p.as_flat_samples()),
            DynamicImage::ImageRgba32F(ref p) => Some(p.as_flat_samples()),
            _ => None,
        }
    }

    /// Return this image's pixels as a native endian byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        image_as_bytes(self)
    }

    /// Return this image's pixels as a byte vector. If the `ImageBuffer`
    /// container is `Vec<u8>`, this operation is free. Otherwise, a copy
    /// is returned.
    pub fn into_bytes(self) -> Vec<u8> {
        image_into_bytes(self)
    }

    /// Return a copy of this image's pixels as a byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        image_to_bytes(self)
    }

    /// Return this image's color type.
    pub fn color(&self) -> color::ColorType {
        match *self {
            DynamicImage::ImageLuma8(_) => color::ColorType::L8,
            DynamicImage::ImageLumaA8(_) => color::ColorType::La8,
            DynamicImage::ImageRgb8(_) => color::ColorType::Rgb8,
            DynamicImage::ImageRgba8(_) => color::ColorType::Rgba8,
            DynamicImage::ImageBgra8(_) => color::ColorType::Bgra8,
            DynamicImage::ImageBgr8(_) => color::ColorType::Bgr8,
            DynamicImage::ImageLuma16(_) => color::ColorType::L16,
            DynamicImage::ImageLumaA16(_) => color::ColorType::La16,
            DynamicImage::ImageRgb16(_) => color::ColorType::Rgb16,
            DynamicImage::ImageRgba16(_) => color::ColorType::Rgba16,
            DynamicImage::ImageRgb32F(_) => color::ColorType::Rgb32F,
            DynamicImage::ImageRgba32F(_) => color::ColorType::Rgba32F,
        }
    }

    /// Returns the width of the underlying image
    pub fn width(&self) -> u32 {
        dynamic_map!(*self, ref p -> {
            p.width()
        })
    }

    /// Returns the height of the underlying image
    pub fn height(&self) -> u32 {
        dynamic_map!(*self, ref p -> {
            p.height()
        })
    }

    /// Return a grayscale version of this image.
    pub fn grayscale(&self) -> DynamicImage {
        match *self {
            DynamicImage::ImageLuma8(ref p) => DynamicImage::ImageLuma8(p.clone()),
            DynamicImage::ImageLumaA8(ref p) => DynamicImage::ImageLuma8(imageops::grayscale(p)),
            DynamicImage::ImageRgb8(ref p) => DynamicImage::ImageLuma8(imageops::grayscale(p)),
            DynamicImage::ImageRgba8(ref p) => DynamicImage::ImageLuma8(imageops::grayscale(p)),
            DynamicImage::ImageBgr8(ref p) => DynamicImage::ImageLuma8(imageops::grayscale(p)),
            DynamicImage::ImageBgra8(ref p) => DynamicImage::ImageLuma8(imageops::grayscale(p)),
            DynamicImage::ImageLuma16(ref p) => DynamicImage::ImageLuma16(p.clone()),
            DynamicImage::ImageLumaA16(ref p) => DynamicImage::ImageLuma16(imageops::grayscale(p)),
            DynamicImage::ImageRgb16(ref p) => DynamicImage::ImageLuma16(imageops::grayscale(p)),
            DynamicImage::ImageRgba16(ref p) => DynamicImage::ImageLuma16(imageops::grayscale(p)),
            DynamicImage::ImageRgb32F(ref p) => DynamicImage::ImageLuma32F(imageops::grayscale(p)),
            DynamicImage::ImageRgba32F(ref p) => DynamicImage::ImageLuma32F(imageops::grayscale(p)),
        }
    }

    /// Invert the colors of this image.
    /// This method operates inplace.
    pub fn invert(&mut self) {
        dynamic_map!(*self, ref mut p -> imageops::invert(p))
    }

    /// Resize this image using the specified filter algorithm.
    /// Returns a new image. The image's aspect ratio is preserved.
    /// The image is scaled to the maximum possible size that fits
    /// within the bounds specified by ```nwidth``` and ```nheight```.
    pub fn resize(&self, nwidth: u32, nheight: u32, filter: imageops::FilterType) -> DynamicImage {
        let (width2, height2) =
            resize_dimensions(self.width(), self.height(), nwidth, nheight, false);

        self.resize_exact(width2, height2, filter)
    }

    /// Resize this image using the specified filter algorithm.
    /// Returns a new image. Does not preserve aspect ratio.
    /// ```nwidth``` and ```nheight``` are the new image's dimensions
    pub fn resize_exact(
        &self,
        nwidth: u32,
        nheight: u32,
        filter: imageops::FilterType,
    ) -> DynamicImage {
        dynamic_map!(*self, ref p => imageops::resize(p, nwidth, nheight, filter))
    }

    /// Scale this image down to fit within a specific size.
    /// Returns a new image. The image's aspect ratio is preserved.
    /// The image is scaled to the maximum possible size that fits
    /// within the bounds specified by ```nwidth``` and ```nheight```.
    ///
    /// This method uses a fast integer algorithm where each source
    /// pixel contributes to exactly one target pixel.
    /// May give aliasing artifacts if new size is close to old size.
    pub fn thumbnail(&self, nwidth: u32, nheight: u32) -> DynamicImage {
        let (width2, height2) =
            resize_dimensions(self.width(), self.height(), nwidth, nheight, false);
        self.thumbnail_exact(width2, height2)
    }

    /// Scale this image down to a specific size.
    /// Returns a new image. Does not preserve aspect ratio.
    /// ```nwidth``` and ```nheight``` are the new image's dimensions.
    /// This method uses a fast integer algorithm where each source
    /// pixel contributes to exactly one target pixel.
    /// May give aliasing artifacts if new size is close to old size.
    pub fn thumbnail_exact(&self, nwidth: u32, nheight: u32) -> DynamicImage {
        dynamic_map!(*self, ref p => imageops::thumbnail(p, nwidth, nheight))
    }

    /// Resize this image using the specified filter algorithm.
    /// Returns a new image. The image's aspect ratio is preserved.
    /// The image is scaled to the maximum possible size that fits
    /// within the larger (relative to aspect ratio) of the bounds
    /// specified by ```nwidth``` and ```nheight```, then cropped to
    /// fit within the other bound.
    pub fn resize_to_fill(
        &self,
        nwidth: u32,
        nheight: u32,
        filter: imageops::FilterType,
    ) -> DynamicImage {
        let (width2, height2) =
            resize_dimensions(self.width(), self.height(), nwidth, nheight, true);

        let mut intermediate = self.resize_exact(width2, height2, filter);
        let (iwidth, iheight) = intermediate.dimensions();
        let ratio = u64::from(iwidth) * u64::from(nheight);
        let nratio = u64::from(nwidth) * u64::from(iheight);

        if nratio > ratio {
            intermediate.crop(0, (iheight - nheight) / 2, nwidth, nheight)
        } else {
            intermediate.crop((iwidth - nwidth) / 2, 0, nwidth, nheight)
        }
    }

    /// Performs a Gaussian blur on this image.
    /// ```sigma``` is a measure of how much to blur by.
    pub fn blur(&self, sigma: f32) -> DynamicImage {
        dynamic_map!(*self, ref p => imageops::blur(p, sigma))
    }

    /// Performs an unsharpen mask on this image.
    /// ```sigma``` is the amount to blur the image by.
    /// ```threshold``` is a control of how much to sharpen.
    ///
    /// See <https://en.wikipedia.org/wiki/Unsharp_masking#Digital_unsharp_masking>
    pub fn unsharpen(&self, sigma: f32, threshold: i32) -> DynamicImage {
        dynamic_map!(*self, ref p => imageops::unsharpen(p, sigma, threshold))
    }

    /// Filters this image with the specified 3x3 kernel.
    pub fn filter3x3(&self, kernel: &[f32]) -> DynamicImage {
        if kernel.len() != 9 {
            panic!("filter must be 3 x 3")
        }

        dynamic_map!(*self, ref p => imageops::filter3x3(p, kernel))
    }

    /// Adjust the contrast of this image.
    /// ```contrast``` is the amount to adjust the contrast by.
    /// Negative values decrease the contrast and positive values increase the contrast.
    pub fn adjust_contrast(&self, c: f32) -> DynamicImage {
        dynamic_map!(*self, ref p => imageops::contrast(p, c))
    }

    /// Brighten the pixels of this image.
    /// ```value``` is the amount to brighten each pixel by.
    /// Negative values decrease the brightness and positive values increase it.
    pub fn brighten(&self, value: i32) -> DynamicImage {
        dynamic_map!(*self, ref p => imageops::brighten(p, value))
    }

    /// Hue rotate the supplied image.
    /// `value` is the degrees to rotate each pixel by.
    /// 0 and 360 do nothing, the rest rotates by the given degree value.
    /// just like the css webkit filter hue-rotate(180)
    pub fn huerotate(&self, value: i32) -> DynamicImage {
        dynamic_map!(*self, ref p => imageops::huerotate(p, value))
    }

    /// Flip this image vertically
    pub fn flipv(&self) -> DynamicImage {
        dynamic_map!(*self, ref p => imageops::flip_vertical(p))
    }

    /// Flip this image horizontally
    pub fn fliph(&self) -> DynamicImage {
        dynamic_map!(*self, ref p => imageops::flip_horizontal(p))
    }

    /// Rotate this image 90 degrees clockwise.
    pub fn rotate90(&self) -> DynamicImage {
        dynamic_map!(*self, ref p => imageops::rotate90(p))
    }

    /// Rotate this image 180 degrees clockwise.
    pub fn rotate180(&self) -> DynamicImage {
        dynamic_map!(*self, ref p => imageops::rotate180(p))
    }

    /// Rotate this image 270 degrees clockwise.
    pub fn rotate270(&self) -> DynamicImage {
        dynamic_map!(*self, ref p => imageops::rotate270(p))
    }

    /// Encode this image and write it to ```w```.
    ///
    /// Assumes the writer is buffered. In most cases,
    /// you should wrap your writer in a `BufWriter` for best performance.
    ///
    /// **Note**: TIFF encoding uses buffered writing,
    /// which can lead to unexpected use of resources
    pub fn write_to<W: Write, F: Into<ImageOutputFormat>>(
        &self,
        w: &mut W,
        format: F,
    ) -> ImageResult<()> {
        #[allow(unused_variables)]
        // When no features are supported
        let w = w;
        #[allow(unused_variables,unused_mut)]
        let mut bytes = self.as_bytes();
        #[allow(unused_variables)]
        let (width, height) = self.dimensions();
        #[allow(unused_variables,unused_mut)]
        let mut color = self.color();
        let format = format.into();

        #[allow(deprecated)]
        match format {
            #[cfg(feature = "png")]
            image::ImageOutputFormat::Png => {
                let p = png::PngEncoder::new(w);
                let converted;
                match *self {
                    DynamicImage::ImageBgra8(_) => {
                        converted = self.to_rgba().into_raw();
                        bytes = &converted;
                        color = color::ColorType::Rgba8;
                    }
                    DynamicImage::ImageBgr8(_) => {
                        converted = self.to_rgb().into_raw();
                        bytes = &converted;
                        color = color::ColorType::Rgb8;
                    }
                    _ => {}
                }
                p.encode(&bytes, width, height, color)?;
                Ok(())
            }

            #[cfg(feature = "pnm")]
            image::ImageOutputFormat::Pnm(subtype) => {
                let mut p = pnm::PnmEncoder::new(w).with_subtype(subtype);
                let converted;
                match *self {
                    DynamicImage::ImageBgra8(_) => {
                        converted = self.to_rgba().into_raw();
                        bytes = &converted;
                        color = color::ColorType::Rgba8;
                    }
                    DynamicImage::ImageBgr8(_) => {
                        converted = self.to_rgb().into_raw();
                        bytes = &converted;
                        color = color::ColorType::Rgb8;
                    }
                    _ => {}
                }
                p.encode(bytes, width, height, color)?;
                Ok(())
            }

            #[cfg(feature = "gif")]
            image::ImageOutputFormat::Gif => {
                let mut g = gif::GifEncoder::new(w);
                g.encode_frame(crate::animation::Frame::new(self.to_rgba()))?;
                Ok(())
            }

            format => write_buffer_with_format(w, bytes, width, height, color, format)
        }
    }

    /// Saves the buffer to a file at the path specified.
    ///
    /// The image format is derived from the file extension.
    pub fn save<Q>(&self, path: Q) -> ImageResult<()>
    where
        Q: AsRef<Path>,
    {
        dynamic_map!(*self, ref p -> {
            p.save(path)
        })
    }

    /// Saves the buffer to a file at the specified path in
    /// the specified format.
    ///
    /// See [`save_buffer_with_format`](fn.save_buffer_with_format.html) for
    /// supported types.
    pub fn save_with_format<Q>(&self, path: Q, format: ImageFormat) -> ImageResult<()>
    where
        Q: AsRef<Path>,
    {
        dynamic_map!(*self, ref p -> {
            p.save_with_format(path, format)
        })
    }
}

impl From<GrayImage> for DynamicImage {
    fn from(image: GrayImage) -> Self {
        DynamicImage::ImageLuma8(image)
    }
}

impl From<GrayAlphaImage> for DynamicImage {
    fn from(image: GrayAlphaImage) -> Self {
        DynamicImage::ImageLumaA8(image)
    }
}

impl From<RgbImage> for DynamicImage {
    fn from(image: RgbImage) -> Self {
        DynamicImage::ImageRgb8(image)
    }
}

impl From<RgbaImage> for DynamicImage {
    fn from(image: RgbaImage) -> Self {
        DynamicImage::ImageRgba8(image)
    }
}

impl From<BgrImage> for DynamicImage {
    fn from(image: BgrImage) -> Self {
        DynamicImage::ImageBgr8(image)
    }
}

impl From<BgraImage> for DynamicImage {
    fn from(image: BgraImage) -> Self {
        DynamicImage::ImageBgra8(image)
    }
}

impl From<Gray16Image> for DynamicImage {
    fn from(image: Gray16Image) -> Self {
        DynamicImage::ImageLuma16(image)
    }
}

impl From<GrayAlpha16Image> for DynamicImage {
    fn from(image: GrayAlpha16Image) -> Self {
        DynamicImage::ImageLumaA16(image)
    }
}

impl From<Rgb16Image> for DynamicImage {
    fn from(image: Rgb16Image) -> Self {
        DynamicImage::ImageRgb16(image)
    }
}

impl From<Rgba16Image> for DynamicImage {
    fn from(image: Rgba16Image) -> Self {
        DynamicImage::ImageRgba16(image)
    }
}

impl From<Rgb32FImage> for DynamicImage {
    fn from(image: Rgb32FImage) -> Self {
        DynamicImage::ImageRgb32F(image)
    }
}

impl From<Rgba32FImage> for DynamicImage {
    fn from(image: Rgba32FImage) -> Self {
        DynamicImage::ImageRgba32F(image)
    }
}

#[allow(deprecated)]
impl GenericImageView for DynamicImage {
    type Pixel = color::Rgba<u8>;
    type InnerImageView = Self;

    fn dimensions(&self) -> (u32, u32) {
        dynamic_map!(*self, ref p -> p.dimensions())
    }

    fn bounds(&self) -> (u32, u32, u32, u32) {
        dynamic_map!(*self, ref p -> p.bounds())
    }

    fn get_pixel(&self, x: u32, y: u32) -> color::Rgba<u8> {
        dynamic_map!(*self, ref p -> p.get_pixel(x, y).to_rgba().into_color())
    }

    fn inner(&self) -> &Self::InnerImageView {
        self
    }
}

#[allow(deprecated)]
impl GenericImage for DynamicImage {
    type InnerImage = DynamicImage;

    fn put_pixel(&mut self, x: u32, y: u32, pixel: color::Rgba<u8>) {
        match *self {
            DynamicImage::ImageLuma8(ref mut p) => p.put_pixel(x, y, pixel.to_luma()),
            DynamicImage::ImageLumaA8(ref mut p) => p.put_pixel(x, y, pixel.to_luma_alpha()),
            DynamicImage::ImageRgb8(ref mut p) => p.put_pixel(x, y, pixel.to_rgb()),
            DynamicImage::ImageRgba8(ref mut p) => p.put_pixel(x, y, pixel),
            DynamicImage::ImageBgr8(ref mut p) => p.put_pixel(x, y, pixel.to_bgr()),
            DynamicImage::ImageBgra8(ref mut p) => p.put_pixel(x, y, pixel.to_bgra()),
            DynamicImage::ImageLuma16(ref mut p) => p.put_pixel(x, y, pixel.to_luma().into_color()),
            DynamicImage::ImageLumaA16(ref mut p) => p.put_pixel(x, y, pixel.to_luma_alpha().into_color()),
            DynamicImage::ImageRgb16(ref mut p) => p.put_pixel(x, y, pixel.to_rgb().into_color()),
            DynamicImage::ImageRgba16(ref mut p) => p.put_pixel(x, y, pixel.into_color()),
            DynamicImage::ImageRgb32F(ref mut p) => p.put_pixel(x, y, pixel.to_rgb().into_color()),
            DynamicImage::ImageRgba32F(ref mut p) => p.put_pixel(x, y, pixel.into_color()),
        }
    }
    /// DEPRECATED: Use iterator `pixels_mut` to blend the pixels directly.
    fn blend_pixel(&mut self, x: u32, y: u32, pixel: color::Rgba<u8>) {
        match *self {
            DynamicImage::ImageLuma8(ref mut p) => p.blend_pixel(x, y, pixel.to_luma()),
            DynamicImage::ImageLumaA8(ref mut p) => p.blend_pixel(x, y, pixel.to_luma_alpha()),
            DynamicImage::ImageRgb8(ref mut p) => p.blend_pixel(x, y, pixel.to_rgb()),
            DynamicImage::ImageRgba8(ref mut p) => p.blend_pixel(x, y, pixel),
            DynamicImage::ImageBgr8(ref mut p) => p.blend_pixel(x, y, pixel.to_bgr()),
            DynamicImage::ImageBgra8(ref mut p) => p.blend_pixel(x, y, pixel.to_bgra()),
            DynamicImage::ImageLuma16(ref mut p) => p.blend_pixel(x, y, pixel.to_luma().into_color()),
            DynamicImage::ImageLumaA16(ref mut p) => p.blend_pixel(x, y, pixel.to_luma_alpha().into_color()),
            DynamicImage::ImageRgb16(ref mut p) => p.blend_pixel(x, y, pixel.to_rgb().into_color()),
            DynamicImage::ImageRgba16(ref mut p) => p.blend_pixel(x, y, pixel.into_color()),
            DynamicImage::ImageRgb32F(ref mut p) => p.blend_pixel(x, y, pixel.to_rgb().into_color()),
            DynamicImage::ImageRgba32F(ref mut p) => p.blend_pixel(x, y, pixel.into_color()),
        }
    }

    /// DEPRECATED: Do not use is function: It is unimplemented!
    fn get_pixel_mut(&mut self, _: u32, _: u32) -> &mut color::Rgba<u8> {
        unimplemented!()
    }

    fn inner_mut(&mut self) -> &mut Self::InnerImage {
        self
    }
}

/// Decodes an image and stores it into a dynamic image
fn decoder_to_image<'a, I: ImageDecoder<'a>>(decoder: I) -> ImageResult<DynamicImage> {
    let (w, h) = decoder.dimensions();
    let color_type = decoder.color_type();

    let image = match color_type {
        color::ColorType::Rgb8 => {
            let buf = image::decoder_to_vec(decoder)?;
            ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageRgb8)
        }

        color::ColorType::Rgba8 => {
            let buf = image::decoder_to_vec(decoder)?;
            ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageRgba8)
        }

        color::ColorType::Bgr8 => {
            let buf = image::decoder_to_vec(decoder)?;
            ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageBgr8)
        }

        color::ColorType::Bgra8 => {
            let buf = image::decoder_to_vec(decoder)?;
            ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageBgra8)
        }

        color::ColorType::L8 => {
            let buf = image::decoder_to_vec(decoder)?;
            ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageLuma8)
        }

        color::ColorType::La8 => {
            let buf = image::decoder_to_vec(decoder)?;
            ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageLumaA8)
        }

        color::ColorType::Rgb16 => {
            let buf = image::decoder_to_vec(decoder)?;
            ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageRgb16)
        }

        color::ColorType::Rgba16 => {
            let buf = image::decoder_to_vec(decoder)?;
            ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageRgba16)
        }

        color::ColorType::Rgb32F => {
            let buf = image::decoder_to_vec(decoder)?;
            ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageRgb32F)
        }

        color::ColorType::Rgba32F => {
            let buf = image::decoder_to_vec(decoder)?;
            ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageRgba32F)
        }

        color::ColorType::L16 => {
            let buf = image::decoder_to_vec(decoder)?;
            ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageLuma16)
        }

        color::ColorType::La16 => {
            let buf = image::decoder_to_vec(decoder)?;
            ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageLumaA16)
        }

        _ => return Err(ImageError::Unsupported(UnsupportedError::from_format_and_kind(
            ImageFormatHint::Unknown,
            UnsupportedErrorKind::Color(color_type.into()),
        ))),
    };
    match image {
        Some(image) => Ok(image),
        None => Err(ImageError::Parameter(
            ParameterError::from_kind(ParameterErrorKind::DimensionMismatch)
        ))
    }
}

fn image_to_bytes(image: &DynamicImage) -> Vec<u8> {
    use crate::traits::EncodableLayout;

    match *image {
        // TODO: consider transmuting
        DynamicImage::ImageLuma8(ref a) => a.as_raw().clone(),
        DynamicImage::ImageLumaA8(ref a) => a.as_raw().clone(),
        DynamicImage::ImageRgb8(ref a) => a.as_raw().clone(),
        DynamicImage::ImageRgba8(ref a) => a.as_raw().clone(),
        DynamicImage::ImageBgr8(ref a) => a.as_raw().clone(),
        DynamicImage::ImageBgra8(ref a) => a.as_raw().clone(),
        DynamicImage::ImageLuma16(ref a) => a.as_bytes().to_vec(),
        DynamicImage::ImageLumaA16(ref a) => a.as_bytes().to_vec(),
        DynamicImage::ImageRgb16(ref a) => a.as_bytes().to_vec(),
        DynamicImage::ImageRgba16(ref a) => a.as_bytes().to_vec(),
        DynamicImage::ImageRgb32F(ref a) => a.as_bytes().to_vec(),
        DynamicImage::ImageRgba32F(ref a) => a.as_bytes().to_vec(),
    }
}

fn image_into_bytes(image: DynamicImage) -> Vec<u8> {
    match image {
        DynamicImage::ImageLuma8(a) => a.into_raw(),
        DynamicImage::ImageLumaA8(a) => a.into_raw(),
        DynamicImage::ImageRgb8(a) => a.into_raw(),
        DynamicImage::ImageRgba8(a) => a.into_raw(),
        DynamicImage::ImageBgr8(a) => a.into_raw(),
        DynamicImage::ImageBgra8(a) => a.into_raw(),
        DynamicImage::ImageLuma16(_) => image.to_bytes(),
        DynamicImage::ImageLumaA16(_) => image.to_bytes(),
        DynamicImage::ImageRgb16(_) => image.to_bytes(),
        DynamicImage::ImageRgba16(_) => image.to_bytes(),
        DynamicImage::ImageRgb32F(_) => image.to_bytes(),
        DynamicImage::ImageRgba32F(_) => image.to_bytes(),
    }
}

fn image_as_bytes(image: &DynamicImage) -> &[u8] {
    use bytemuck::cast_slice;
    match image {
        DynamicImage::ImageLuma8(a) => &*a,
        DynamicImage::ImageLumaA8(a) => &*a,
        DynamicImage::ImageRgb8(a) => &*a,
        DynamicImage::ImageRgba8(a) => &*a,
        DynamicImage::ImageBgr8(a) => &*a,
        DynamicImage::ImageBgra8(a) => &*a,
        DynamicImage::ImageLuma16(a) => cast_slice(&*a),
        DynamicImage::ImageLumaA16(a) => cast_slice(&*a),
        DynamicImage::ImageRgb16(a) => cast_slice(&*a),
        DynamicImage::ImageRgba16(a) => cast_slice(&*a),
        DynamicImage::ImageRgb32F(a) => cast_slice(&*a),
        DynamicImage::ImageRgba32F(a) => cast_slice(&*a),
    }
}

/// Open the image located at the path specified.
/// The image's format is determined from the path's file extension.
///
/// Try [`io::Reader`] for more advanced uses, including guessing the format based on the file's
/// content before its path.
///
/// [`io::Reader`]: io/struct.Reader.html
pub fn open<P>(path: P) -> ImageResult<DynamicImage>
where
    P: AsRef<Path>,
{
    // thin wrapper function to strip generics before calling open_impl
    free_functions::open_impl(path.as_ref())
}

/// Read the dimensions of the image located at the specified path.
/// This is faster than fully loading the image and then getting its dimensions.
///
/// Try [`io::Reader`] for more advanced uses, including guessing the format based on the file's
/// content before its path or manually supplying the format.
///
/// [`io::Reader`]: io/struct.Reader.html
pub fn image_dimensions<P>(path: P) -> ImageResult<(u32, u32)>
where
    P: AsRef<Path>,
{
    // thin wrapper function to strip generics before calling open_impl
    free_functions::image_dimensions_impl(path.as_ref())
}

/// Saves the supplied buffer to a file at the path specified.
///
/// The image format is derived from the file extension. The buffer is assumed to have
/// the correct format according to the specified color type.
///
/// This will lead to corrupted files if the buffer contains malformed data. Currently only
/// jpeg, png, ico, pnm, bmp, exr and tiff files are supported.
pub fn save_buffer<P>(
    path: P,
    buf: &[u8],
    width: u32,
    height: u32,
    color: color::ColorType,
) -> ImageResult<()>
where
    P: AsRef<Path>,
{
    // thin wrapper function to strip generics before calling save_buffer_impl
    free_functions::save_buffer_impl(path.as_ref(), buf, width, height, color)
}

/// Saves the supplied buffer to a file at the path specified
/// in the specified format.
///
/// The buffer is assumed to have the correct format according
/// to the specified color type.
/// This will lead to corrupted files if the buffer contains
/// malformed data. Currently only jpeg, png, ico, bmp, exr and
/// tiff files are supported.
pub fn save_buffer_with_format<P>(
    path: P,
    buf: &[u8],
    width: u32,
    height: u32,
    color: color::ColorType,
    format: ImageFormat,
) -> ImageResult<()>
where
    P: AsRef<Path>,
{
    // thin wrapper function to strip generics
    free_functions::save_buffer_with_format_impl(path.as_ref(), buf, width, height, color, format)
}

/// Writes the supplied buffer to a writer in the specified format.
///
/// The buffer is assumed to have the correct format according
/// to the specified color type.
/// This will lead to corrupted writers if the buffer contains
/// malformed data.
///
/// See [`ImageOutputFormat`](../enum.ImageOutputFormat.html) for
/// supported types.
///
/// Assumes the writer is buffered. In most cases,
/// you should wrap your writer in a `BufWriter` for best performance.
///
/// **Note**: TIFF encoding uses buffered writing,
/// which can lead to unexpected use of resources
pub fn write_buffer_with_format<W, F>(
    writer: &mut W,
    buf: &[u8],
    width: u32,
    height: u32,
    color: color::ColorType,
    format: F,
) -> ImageResult<()>
where
    W: Write,
    F: Into<ImageOutputFormat>,
{
    // thin wrapper function to strip generics
    free_functions::write_buffer_impl(writer, buf, width, height, color, format.into())
}

/// Create a new image from a byte slice
///
/// Makes an educated guess about the image format.
/// TGA is not supported by this function.
///
/// Try [`io::Reader`] for more advanced uses.
///
/// [`io::Reader`]: io/struct.Reader.html
pub fn load_from_memory(buffer: &[u8]) -> ImageResult<DynamicImage> {
    let format = free_functions::guess_format(buffer)?;
    load_from_memory_with_format(buffer, format)
}

/// Create a new image from a byte slice
///
/// This is just a simple wrapper that constructs an `std::io::Cursor` around the buffer and then
/// calls `load` with that reader.
///
/// Try [`io::Reader`] for more advanced uses.
///
/// [`load`]: fn.load.html
/// [`io::Reader`]: io/struct.Reader.html
#[inline(always)]
pub fn load_from_memory_with_format(buf: &[u8], format: ImageFormat) -> ImageResult<DynamicImage> {
    let b = io::Cursor::new(buf);
    free_functions::load(b, format)
}

#[cfg(test)]
mod bench {
    #[cfg(feature = "benchmarks")]
    use test;

    #[bench]
    #[cfg(feature = "benchmarks")]
    fn bench_conversion(b: &mut test::Bencher) {
        let a = super::DynamicImage::ImageRgb8(crate::ImageBuffer::new(1000, 1000));
        b.iter(|| a.to_luma8());
        b.bytes = 1000 * 1000 * 3
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_empty_file() {
        assert!(super::load_from_memory(b"").is_err());
    }

    #[cfg(feature = "jpeg")]
    #[test]
    fn image_dimensions() {
        let im_path = "./tests/images/jpg/progressive/cat.jpg";
        let dims = super::image_dimensions(im_path).unwrap();
        assert_eq!(dims, (320, 240));
    }

    #[cfg(feature = "png")]
    #[test]
    fn open_16bpc_png() {
        let im_path = "./tests/images/png/16bpc/basn6a16.png";
        let image = super::open(im_path).unwrap();
        assert_eq!(image.color(), super::color::ColorType::Rgba16);
    }
}
