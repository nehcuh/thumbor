use std::io::Cursor;

use image::{DynamicImage, ImageFormat, Rgb, RgbaImage};
use lazy_static::lazy_static;

use super::{Engine, SpecTransform};
use crate::pb::{
    abi::{
        Contrast, Crop, Filter, Fliph, Flipv, Resize, Spec, Watermark, filter::Filter as SubFilter,
        resize,
    },
    mix_with_color,
};

pub struct ImageEngine(DynamicImage);

lazy_static! {
    static ref WATERMARK: DynamicImage = {
        let data = include_bytes!("../../rust-logo.png");
        let watermark = image::load_from_memory(data).unwrap();
        watermark.resize(64, 64, image::imageops::FilterType::Nearest)
    };
}

impl Engine for ImageEngine {
    fn apply(&mut self, specs: &[Spec]) {
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
        self.0
            .write_to(&mut writer, format)
            .expect("Failed to write image to buffer");
        buf
    }
}

impl SpecTransform<&Crop> for ImageEngine {
    fn transform(&mut self, op: &Crop) {
        let width = op.x2 - op.x1;
        let height = op.y2 - op.y1;
        let cropped_img = image::imageops::crop_imm(&self.0, op.x1, op.y1, width, height);
        self.0 = DynamicImage::ImageRgba8(cropped_img.to_image());
    }
}

impl SpecTransform<&Contrast> for ImageEngine {
    fn transform(&mut self, op: &Contrast) {
        self.0 = image::DynamicImage::ImageRgba8(image::imageops::contrast(&self.0, op.contrast));
    }
}

impl SpecTransform<&Resize> for ImageEngine {
    fn transform(&mut self, op: &Resize) {
        match resize::ResizeType::try_from(op.rtype).unwrap() {
            resize::ResizeType::Normal => {
                self.0 = image::DynamicImage::ImageRgba8(image::imageops::resize(
                    &self.0,
                    op.width,
                    op.height,
                    resize::SampleFilter::try_from(op.filter).unwrap().into(),
                ));
            }
            resize::ResizeType::SeamCarve => {
                // original from photon_rs: https://docs.rs/photon-rs/0.3.2/src/photon_rs/transform.rs.html#296-326
                let mut img: RgbaImage = self.0.to_rgba8().into();
                let (w, h) = img.dimensions();
                let (diff_w, diff_h) = (w - w.min(op.width), h - h.min(op.height));

                for _ in 0..diff_w {
                    let vec_steam = imageproc::seam_carving::find_vertical_seam(&img);
                    img = imageproc::seam_carving::remove_vertical_seam(&img, &vec_steam);
                }
                if diff_h.ne(&0_u32) {
                    img = image::imageops::rotate90(&img);
                    for _ in 0..diff_h {
                        let vec_steam = imageproc::seam_carving::find_vertical_seam(&img);
                        img = imageproc::seam_carving::remove_vertical_seam(&img, &vec_steam);
                    }
                    img = image::imageops::rotate270(&img);
                }
                self.0 = image::DynamicImage::ImageRgba8(img)
            }
        }
    }
}

impl SpecTransform<&Filter> for ImageEngine {
    fn transform(&mut self, op: &Filter) {
        match SubFilter::try_from(op.filter).unwrap() {
            SubFilter::Unspecified => mix_with_color(&mut self.0, Rgb([0, 89, 173]), 0.2),
            SubFilter::Oceanic => mix_with_color(&mut self.0, Rgb([0, 89, 173]), 0.2),
            SubFilter::Islands => mix_with_color(&mut self.0, Rgb([0, 24, 95]), 0.2),
            SubFilter::Marine => mix_with_color(&mut self.0, Rgb([0, 14, 119]), 0.2),
        }
    }
}

impl SpecTransform<&Fliph> for ImageEngine {
    fn transform(&mut self, _op: &Fliph) {
        image::imageops::flip_horizontal_in_place(&mut self.0);
    }
}

impl SpecTransform<&Flipv> for ImageEngine {
    fn transform(&mut self, _op: &Flipv) {
        image::imageops::flip_vertical_in_place(&mut self.0);
    }
}

impl SpecTransform<&Watermark> for ImageEngine {
    fn transform(&mut self, op: &Watermark) {
        image::imageops::overlay(&mut self.0, &*WATERMARK, op.x as i64, op.y as i64);
    }
}
