use std::path::PathBuf;

use crate::{core::copy::CopyToContainerCollection, BuildableImage, CopyToContainer, GenericImage};

#[derive(Debug)]
pub struct GenericBuildableImage {
    name: String,
    tag: String,
    build_context: CopyToContainerCollection,
}

impl GenericBuildableImage {
    pub fn new(name: impl Into<String>, tag: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tag: tag.into(),
            build_context: CopyToContainerCollection::new(vec![]),
        }
    }

    pub fn with_dockerfile(self, source: impl Into<PathBuf>) -> Self {
        self.with_file(source.into(), "Dockerfile")
    }

    pub fn with_dockerfile_string(self, content: impl Into<String>) -> Self {
        self.with_data(content.into(), "Dockerfile")
    }

    pub fn with_file(mut self, source: impl Into<PathBuf>, target: impl Into<String>) -> Self {
        self.build_context
            .add(CopyToContainer::new(source.into(), target));
        self
    }

    pub fn with_data(mut self, data: impl Into<Vec<u8>>, target: impl Into<String>) -> Self {
        self.build_context
            .add(CopyToContainer::new(data.into(), target));
        self
    }
}

impl BuildableImage for GenericBuildableImage {
    type Built = GenericImage;

    fn build_context(&self) -> CopyToContainerCollection {
        self.build_context.clone()
    }

    fn descriptor(&self) -> String {
        format!("{}:{}", self.name, self.tag)
    }

    fn into_image(self) -> Self::Built {
        GenericImage::new(&self.name, &self.tag)
    }
}
