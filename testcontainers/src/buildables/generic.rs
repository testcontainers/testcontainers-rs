use std::path::PathBuf;

use crate::{
    core::{copy::CopyToContainerCollection, BuildContextBuilder},
    BuildableImage, GenericImage,
};

#[derive(Debug)]
pub struct GenericBuildableImage {
    name: String,
    tag: String,
    build_context_builder: BuildContextBuilder,
}

impl GenericBuildableImage {
    pub fn new(name: impl Into<String>, tag: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tag: tag.into(),
            build_context_builder: BuildContextBuilder::default(),
        }
    }

    pub fn with_dockerfile(mut self, source: impl Into<PathBuf>) -> Self {
        self.build_context_builder = self.build_context_builder.with_dockerfile(source);
        self
    }

    pub fn with_dockerfile_string(mut self, content: impl Into<String>) -> Self {
        self.build_context_builder = self.build_context_builder.with_dockerfile_string(content);
        self
    }

    pub fn with_file(mut self, source: impl Into<PathBuf>, target: impl Into<String>) -> Self {
        self.build_context_builder = self.build_context_builder.with_file(source, target);
        self
    }

    pub fn with_data(mut self, data: impl Into<Vec<u8>>, target: impl Into<String>) -> Self {
        self.build_context_builder = self.build_context_builder.with_data(data, target);
        self
    }
}

impl BuildableImage for GenericBuildableImage {
    type Built = GenericImage;

    fn build_context(&self) -> CopyToContainerCollection {
        self.build_context_builder.as_copy_to_container_collection()
    }

    fn descriptor(&self) -> String {
        format!("{}:{}", self.name, self.tag)
    }

    fn into_image(self) -> Self::Built {
        GenericImage::new(&self.name, &self.tag)
    }
}
