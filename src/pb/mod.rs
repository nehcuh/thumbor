pub(crate) mod abi;
use base64::Engine;
use image::{DynamicImage, Rgb};
use prost::Message;

impl abi::ImageSpec {
    pub fn new(specs: Vec<abi::Spec>) -> Self {
        Self { specs }
    }
}

impl From<&abi::ImageSpec> for String {
    fn from(value: &abi::ImageSpec) -> Self {
        let data = value.encode_to_vec();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
    }
}

impl TryFrom<&str> for abi::ImageSpec {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let data = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(value)?;
        Ok(abi::ImageSpec::decode(&data[..])?)
    }
}

impl From<abi::resize::SampleFilter> for image::imageops::FilterType {
    fn from(value: abi::resize::SampleFilter) -> Self {
        match value {
            abi::resize::SampleFilter::Undefined => image::imageops::FilterType::Nearest,
            abi::resize::SampleFilter::Nereast => image::imageops::FilterType::Nearest,
            abi::resize::SampleFilter::Triangle => image::imageops::FilterType::Triangle,
            abi::resize::SampleFilter::CatmullRom => image::imageops::FilterType::CatmullRom,
            abi::resize::SampleFilter::Gaussian => image::imageops::FilterType::Gaussian,
            abi::resize::SampleFilter::Lanczos3 => image::imageops::FilterType::Lanczos3,
        }
    }
}

impl abi::filter::Filter {
    pub fn apply(self, img: &mut DynamicImage) {
        match self {
            abi::filter::Filter::Unspecified => {}
            abi::filter::Filter::Oceanic => mix_with_color(img, Rgb([0, 89, 173]), 0.2),
            abi::filter::Filter::Islands => mix_with_color(img, Rgb([0, 24, 95]), 0.2),
            abi::filter::Filter::Marine => mix_with_color(img, Rgb([0, 14, 119]), 0.2),
        }
    }
}

pub fn mix_with_color(img: &mut DynamicImage, mix_color: Rgb<u8>, opacity: f32) {
    // 确保 img 可转换成 RGB8 格式
    if let Some(rgb_img) = img.as_mut_rgb8() {
        // 限制 opacity 在有效范围内 [0.0, 1.0]
        let opacity = opacity.clamp(0.0, 1.0);

        // 预先计算混合颜色的加权值和原始像素的加权因子
        let mix_red_offset = mix_color[0] as f32 * opacity;
        let mix_green_offset = mix_color[1] as f32 * opacity;
        let mix_blue_offset = mix_color[2] as f32 * opacity;
        let factor = 1.0 - opacity; // 原始像素的权重

        for pixel in rgb_img.pixels_mut() {
            let current_r = pixel[0] as f32;
            let current_g = pixel[1] as f32;
            let current_b = pixel[2] as f32;
            // alpha 通道保持不变

            let new_r = mix_red_offset + current_r * factor;
            let new_g = mix_green_offset + current_g * factor;
            let new_b = mix_blue_offset + current_b * factor;

            // 更新像素数据（确保值在 [0,255] 内）
            pixel[0] = new_r.clamp(0.0, 255.0) as u8;
            pixel[1] = new_g.clamp(0.0, 255.0) as u8;
            pixel[2] = new_b.clamp(0.0, 255.0) as u8;
        }
    }
}

impl abi::Spec {
    pub fn new_resize_seam_carve(width: u32, height: u32) -> Self {
        Self {
            data: Some(abi::spec::Data::Resize(abi::Resize {
                width,
                height,
                rtype: abi::resize::ResizeType::SeamCarve as i32,
                filter: abi::resize::SampleFilter::Undefined as i32,
            })),
        }
    }

    pub fn new_resize(width: u32, height: u32, filter: abi::resize::SampleFilter) -> Self {
        Self {
            data: Some(abi::spec::Data::Resize(abi::Resize {
                width,
                height,
                rtype: abi::resize::ResizeType::Normal as i32,
                filter: filter as i32,
            })),
        }
    }

    pub fn new_filter(filter: abi::filter::Filter) -> Self {
        Self {
            data: Some(abi::spec::Data::Filter(abi::Filter {
                filter: filter as i32,
            })),
        }
    }

    pub fn new_watermark(x: u32, y: u32) -> Self {
        Self {
            data: Some(abi::spec::Data::Watermark(abi::Watermark { x, y })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Borrow;

    #[test]
    fn test_encoded_spec_could_be_decoded() {
        let spec1 = abi::Spec::new_resize(600, 600, abi::resize::SampleFilter::CatmullRom);
        let spec2 = abi::Spec::new_filter(abi::filter::Filter::Marine);
        let image_spec = abi::ImageSpec::new(vec![spec1, spec2]);
        let s: String = image_spec.borrow().into();
        assert_eq!(image_spec, s.as_str().try_into().unwrap())
    }
}
