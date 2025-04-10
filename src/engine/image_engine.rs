use std::io::Cursor;

use anyhow::Result as AnyResult;
use bytes::Bytes;
use image::{DynamicImage, ImageFormat};
use imageproc::drawing::Canvas;
use lazy_static::lazy_static;

use super::SpecTransform;
pub struct ImageEngine(DynamicImage);

lazy_static! {
    static ref WATERMARK: DynamicImage = {
        let data = include_bytes!("../../rust-logo.png");
        let watermark = image::load_from_memory(data).unwrap();
        watermark.resize(64, 64, image::imageops::FilterType::Nearest)
    };
}

impl TryFrom<Bytes> for ImageEngine {
    type Error = anyhow::Error;

    fn try_from(value: Bytes) -> AnyResult<Self> {
        let img = image::load_from_memory(value.as_ref())?;
        Ok(ImageEngine(img))
    }
}

impl super::Engine for ImageEngine {
    fn apply(&mut self, specs: &[crate::pb::abi::Spec]) {
        for spec in specs.iter() {
            match spec.data {
                None => {}
                Some(crate::pb::abi::spec::Data::Crop(ref v)) => self.transform(v),
                Some(crate::pb::abi::spec::Data::Resize(ref v)) => self.transform(v),
                Some(crate::pb::abi::spec::Data::Contrast(ref v)) => self.transform(v),
                Some(crate::pb::abi::spec::Data::Filter(ref v)) => self.transform(v),
                Some(crate::pb::abi::spec::Data::Fliph(ref v)) => self.transform(v),
                Some(crate::pb::abi::spec::Data::Flipv(ref v)) => self.transform(v),
                Some(crate::pb::abi::spec::Data::Watermark(ref v)) => self.transform(v),
            }
        }
    }

    fn generate(self, format: ImageFormat) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1024);
        let mut writer = Cursor::new(&mut buf);
        let img = if format == ImageFormat::Jpeg {
            DynamicImage::ImageRgb8(self.0.to_rgb8())
        } else {
            self.0
        };

        img.write_to(&mut writer, format)
            .expect("Failed to write image to buffer");
        // self.0
        //     .write_to(&mut writer, format)
        //     .expect("Failed to write image to buffer");
        buf
    }
}

impl super::SpecTransform<&crate::pb::abi::Crop> for ImageEngine {
    fn transform(&mut self, op: &crate::pb::abi::Crop) {
        let x1 = op.x1.min(self.0.width());
        let y1 = op.y1.min(self.0.height());
        let x2 = op.x2.min(self.0.width());
        let y2 = op.y2.min(self.0.height());

        // Make sure x2 > x1 and y2 > y1
        if x2 <= x1 || y2 <= y1 {
            return; // Invalid crop dimensions - do nothing
        }
        let width = x2 - x1;
        let height = y2 - y1;
        let cropped_img = image::imageops::crop_imm(&self.0, op.x1, op.y1, width, height);
        self.0 = DynamicImage::ImageRgba8(cropped_img.to_image());
    }
}

impl super::SpecTransform<&crate::pb::abi::Contrast> for ImageEngine {
    fn transform(&mut self, op: &crate::pb::abi::Contrast) {
        self.0 = image::DynamicImage::ImageRgba8(image::imageops::contrast(&self.0, op.contrast));
    }
}

impl super::SpecTransform<&crate::pb::abi::Resize> for ImageEngine {
    fn transform(&mut self, op: &crate::pb::abi::Resize) {
        match crate::pb::abi::resize::ResizeType::try_from(op.rtype).unwrap() {
            crate::pb::abi::resize::ResizeType::Normal => {
                self.0 = image::DynamicImage::ImageRgba8(image::imageops::resize(
                    &self.0,
                    op.width,
                    op.height,
                    crate::pb::abi::resize::SampleFilter::try_from(op.filter)
                        .unwrap()
                        .into(),
                ));
            }
            crate::pb::abi::resize::ResizeType::SeamCarve => {
                // original from photon_rs: https://docs.rs/photon-rs/0.3.2/src/photon_rs/transform.rs.html#296-326
                let (w, h) = self.0.dimensions();
                let (diff_w, diff_h) = (w - w.min(op.width), h - h.min(op.height));

                for _ in 0..diff_w {
                    let vec_steam =
                        imageproc::seam_carving::find_vertical_seam(&self.0.to_rgba8().into());
                    self.0 = imageproc::seam_carving::remove_vertical_seam(
                        &self.0.to_rgba8().into(),
                        &vec_steam,
                    )
                    .into();
                }
                if diff_h.ne(&0_u32) {
                    self.0 = image::imageops::rotate90(&self.0.to_rgba8()).into();
                    for _ in 0..diff_h {
                        let vec_steam =
                            imageproc::seam_carving::find_vertical_seam(&self.0.to_rgba8().into());
                        self.0 = imageproc::seam_carving::remove_vertical_seam(
                            &self.0.to_rgba8().into(),
                            &vec_steam,
                        )
                        .into();
                    }
                    self.0 = image::imageops::rotate270(&self.0.to_rgba8()).into();
                }
            }
        }
    }
}

impl super::SpecTransform<&crate::pb::abi::Filter> for ImageEngine {
    fn transform(&mut self, op: &crate::pb::abi::Filter) {
        let filter_type = crate::pb::abi::filter::Filter::try_from(op.filter).unwrap();
        filter_type.apply(&mut self.0);
    }
}

impl SpecTransform<&crate::pb::abi::Fliph> for ImageEngine {
    fn transform(&mut self, _op: &crate::pb::abi::Fliph) {
        image::imageops::flip_horizontal_in_place(&mut self.0);
    }
}

impl SpecTransform<&crate::pb::abi::Flipv> for ImageEngine {
    fn transform(&mut self, _op: &crate::pb::abi::Flipv) {
        image::imageops::flip_vertical_in_place(&mut self.0);
    }
}

impl SpecTransform<&crate::pb::abi::Watermark> for ImageEngine {
    fn transform(&mut self, op: &crate::pb::abi::Watermark) {
        image::imageops::overlay(&mut self.0, &*WATERMARK, op.x as i64, op.y as i64);
    }
}
