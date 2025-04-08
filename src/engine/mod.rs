use image::ImageFormat;

pub(crate) mod image_engine;

pub trait Engine {
    fn apply(&mut self, specs: &[crate::pb::abi::Spec]);
    fn generate(self, format: ImageFormat) -> Vec<u8>;
}

pub trait SpecTransform<T> {
    fn transform(&mut self, op: T);
}
