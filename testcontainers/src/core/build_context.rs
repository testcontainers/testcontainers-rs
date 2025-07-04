use std::path::PathBuf;

use crate::{core::copy::CopyToContainerCollection, CopyToContainer};

#[derive(Debug, Default, Clone)]
pub struct BuildContextBuilder {
    build_context_parts: Vec<CopyToContainer>,
}

impl BuildContextBuilder {
    pub fn with_dockerfile(self, source: impl Into<PathBuf>) -> Self {
        self.with_file(source.into(), "Dockerfile")
    }

    pub fn with_dockerfile_string(self, content: impl Into<String>) -> Self {
        self.with_data(content.into(), "Dockerfile")
    }

    pub fn with_file(mut self, source: impl Into<PathBuf>, target: impl Into<String>) -> Self {
        self.build_context_parts
            .push(CopyToContainer::new(source.into(), target));
        self
    }

    pub fn with_data(mut self, data: impl Into<Vec<u8>>, target: impl Into<String>) -> Self {
        self.build_context_parts
            .push(CopyToContainer::new(data.into(), target));
        self
    }

    pub fn collect(self) -> CopyToContainerCollection {
        CopyToContainerCollection::new(self.build_context_parts)
    }

    pub fn as_copy_to_container_collection(&self) -> CopyToContainerCollection {
        CopyToContainerCollection::new(self.build_context_parts.clone())
    }
}
