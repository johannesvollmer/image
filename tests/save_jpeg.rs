//! Test saving "default" and specific quality jpeg.
#![cfg(all(feature = "jpeg", feature = "tiff"))]
extern crate image;

use image::{ImageOutputFormat, ImageFormat};
use std::convert::TryFrom;

#[test]
fn jqeg_qualitys() {
    let img = image::open("tests/images/tiff/testsuite/mandrill.tiff").unwrap();

    let mut default = vec![];
    img.write_to(&mut default, ImageOutputFormat::try_from(ImageFormat::Jpeg).unwrap()).unwrap();
    assert_eq!(&[255, 216], &default[..2]);

    let mut small = vec![];
    img.write_to(&mut small, ImageOutputFormat::Jpeg(10))
        .unwrap();
    assert_eq!(&[255, 216], &small[..2]);

    assert!(small.len() < default.len());

    let mut large = vec![];
    img.write_to(&mut large, ImageOutputFormat::Jpeg(99))
        .unwrap();
    assert_eq!(&[255, 216], &large[..2]);

    assert!(large.len() > default.len());
}
