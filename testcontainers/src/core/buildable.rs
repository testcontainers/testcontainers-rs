use crate::{core::copy::CopyToContainerCollection, Image};

pub trait BuildableImage {
    type Built: Image;

    fn build_context(&self) -> CopyToContainerCollection;
    fn descriptor(&self) -> String;

    fn into_image(self) -> Self::Built;
}
