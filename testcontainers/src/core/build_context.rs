use std::path::PathBuf;

use crate::{core::copy::CopyToContainerCollection, CopyToContainer};

/// Builder for managing Docker BuildKit build contexts.
///
/// A build context contains all the files and data that Docker needs to build an image.
/// This includes the Dockerfile, source code, configuration files, and any other materials
/// referenced by the Dockerfile's `COPY` or `ADD` instructions.
/// More information see: <https://docs.docker.com/build/concepts/context/>
///
/// The `BuildContextBuilder` collects these materials and packages them into a TAR archive
/// that can be sent to the Docker daemon for building.
///
/// # Example  
///   
/// ```rust,no_run
/// use testcontainers::core::BuildContextBuilder;
///
/// let context = BuildContextBuilder::default()  
///     .with_dockerfile_string("FROM alpine:latest\nCOPY app /usr/local/bin/")  
///     .with_file("./target/release/app", "./app")  
///     .with_data(b"#!/bin/sh\necho 'Hello World'", "./hello.sh")  
///     .collect();  
/// ```  
#[derive(Debug, Default, Clone)]
pub struct BuildContextBuilder {
    build_context_parts: Vec<CopyToContainer>,
}

impl BuildContextBuilder {
    /// Adds a Dockerfile from the filesystem to the build context.
    ///
    /// # Arguments
    ///
    /// * `source` - Path to the Dockerfile on the local filesystem
    pub fn with_dockerfile(self, source: impl Into<PathBuf>) -> Self {
        self.with_file(source.into(), "Dockerfile")
    }

    /// Adds a Dockerfile from a string to the build context.
    ///
    /// This is useful for generating Dockerfiles programmatically or embedding
    /// simple Dockerfiles directly in test code.
    ///
    /// # Arguments
    ///
    /// * `content` - The complete Dockerfile content as a string
    pub fn with_dockerfile_string(self, content: impl Into<String>) -> Self {
        self.with_data(content.into(), "Dockerfile")
    }

    /// Adds a file or directory from the filesystem to the build context.
    ///
    /// Be aware, that if you don't add the Dockerfile with the specific `with_dockerfile()`
    /// or `with_dockerfile_string()` functions it has to be named `Dockerfile`in the build
    /// context. Containerfile won't be recognized!
    ///
    /// # Arguments
    ///
    /// * `source` - Path to the file or directory on the local filesystem
    /// * `target` - Path where the file should be placed in the build context
    pub fn with_file(mut self, source: impl Into<PathBuf>, target: impl Into<String>) -> Self {
        self.build_context_parts
            .push(CopyToContainer::new(source.into(), target));
        self
    }

    /// Adds data directly to the build context as a file.
    ///
    /// This method allows you to embed file content directly without requiring
    /// files to exist on the filesystem. Useful for generated content, templates,
    /// or small configuration files.
    ///
    /// # Arguments
    ///
    /// * `data` - The file content as bytes
    /// * `target` - Path where the file should be placed in the build context
    pub fn with_data(mut self, data: impl Into<Vec<u8>>, target: impl Into<String>) -> Self {
        self.build_context_parts
            .push(CopyToContainer::new(data.into(), target));
        self
    }

    /// Consumes the builder and returns the collected build context.  
    ///   
    /// This method finalizes the build context and returns a [`CopyToContainerCollection`]  
    /// that can be converted to a TAR archive for Docker.  
    ///   
    /// # Returns  
    ///   
    /// A [`CopyToContainerCollection`] containing all the build context materials.  
    pub fn collect(self) -> CopyToContainerCollection {
        CopyToContainerCollection::new(self.build_context_parts)
    }

    /// Returns the build context without consuming the builder.
    ///
    /// This method creates a clone of the current build context state, allowing
    /// the builder to be reused or modified further.
    ///
    /// # Returns
    ///
    /// A [`CopyToContainerCollection`] containing all the current build context materials.
    pub fn as_copy_to_container_collection(&self) -> CopyToContainerCollection {
        CopyToContainerCollection::new(self.build_context_parts.clone())
    }
}
