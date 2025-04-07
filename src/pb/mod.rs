pub(crate) mod abi;
use abi::{ImageSpec, Spec, filter::Filter, resize::SampleFilter};
use base64::Engine;
use image::{DynamicImage, Rgb};
use prost::Message;

impl ImageSpec {
    pub fn new(specs: Vec<Spec>) -> Self {
        Self { specs }
    }
}

impl From<&ImageSpec> for String {
    fn from(value: &ImageSpec) -> Self {
        let data = value.encode_to_vec();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
    }
}

impl TryFrom<&str> for ImageSpec {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let data = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(value)?;
        Ok(ImageSpec::decode(&data[..])?)
    }
}

impl From<SampleFilter> for image::imageops::FilterType {
    fn from(value: SampleFilter) -> Self {
        match value {
            SampleFilter::Undefined => image::imageops::FilterType::Nearest,
            SampleFilter::Nereast => image::imageops::FilterType::Nearest,
            SampleFilter::Triangle => image::imageops::FilterType::Triangle,
            SampleFilter::CatmullRom => image::imageops::FilterType::CatmullRom,
            SampleFilter::Gaussian => image::imageops::FilterType::Gaussian,
            SampleFilter::Lanczos3 => image::imageops::FilterType::Lanczos3,
        }
    }
}

pub fn mix_with_color(img: &mut DynamicImage, mix_color: Rgb<u8>, opacity: f32) {
    let img = img.as_mut_rgb8();
    // 限制 opacity 在有效范围内 [0.0, 1.0]
    let opacity = opacity.clamp(0.0, 1.0);

    // 预先计算混合颜色的加权值和原始像素的加权因子
    let mix_red_offset = mix_color[0] as f32 * opacity;
    let mix_green_offset = mix_color[1] as f32 * opacity;
    let mix_blue_offset = mix_color[2] as f32 * opacity;
    let factor = 1.0 - opacity; // 原始像素的权重

    // 遍历每一个可变像素
    for pixel in img.unwrap().pixels_mut() {
        // pixel 是 &mut Rgba<u8>
        // 获取当前像素的 RGB 值 (作为 f32)
        let current_r = pixel[0] as f32;
        let current_g = pixel[1] as f32;
        let current_b = pixel[2] as f32;
        // Alpha 通道保持不变
        // let current_a = pixel[3];

        // 计算混合后的 RGB 值
        // new_value = (mix_color * opacity) + (current_pixel * (1.0 - opacity))
        let new_r = mix_red_offset + current_r * factor;
        let new_g = mix_green_offset + current_g * factor;
        let new_b = mix_blue_offset + current_b * factor;

        // 更新像素数据，确保值在 [0, 255] 范围内并转换为 u8
        pixel[0] = new_r.clamp(0.0, 255.0) as u8;
        pixel[1] = new_g.clamp(0.0, 255.0) as u8;
        pixel[2] = new_b.clamp(0.0, 255.0) as u8;
        // pixel[3] (alpha) 保持不变
    }
    // 函数修改了传入的 img，无需返回值
}
