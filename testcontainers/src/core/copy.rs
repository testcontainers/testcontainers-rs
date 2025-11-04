use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio_tar::EntryType;

#[derive(Debug, Clone)]
pub struct CopyToContainerCollection(Vec<CopyToContainer>);

#[derive(Debug, Clone)]
pub struct CopyToContainer {
    target: String,
    source: CopyDataSource,
}

#[derive(Debug, Clone)]
pub enum CopyDataSource {
    File(PathBuf),
    Data(Vec<u8>),
}

/// Errors that can occur while materializing data copied from a container.
#[derive(Debug, thiserror::Error)]
pub enum CopyFromContainerError {
    #[error("io failed with error: {0}")]
    Io(#[from] std::io::Error),
    #[error("archive did not contain any regular files")]
    EmptyArchive,
    #[error("archive contained multiple files, but only one was expected")]
    MultipleFilesInArchive,
    #[error("requested container path is a directory")]
    IsDirectory,
    #[error("archive entry type '{0:?}' is not supported for requested target")]
    UnsupportedEntry(EntryType),
}

/// Abstraction for materializing the bytes read from a source into a concrete destination.
///
/// Implementors typically persist the incoming bytes to disk or buffer them in memory and then
/// return a value that callers can work with (for example, the path that was written or the
/// collected bytes). Implementations must consume the provided reader until EOF or return an error.
#[async_trait(?Send)]
pub trait CopyFileFromContainer {
    type Output;

    /// Writes all bytes from the reader into `self`, returning a value that represents the completed operation.
    ///
    /// Implementations may mutate `self` and must propagate I/O errors via [`CopyFromContainerError`].
    async fn copy_from_reader<R>(self, reader: R) -> Result<Self::Output, CopyFromContainerError>
    where
        R: AsyncRead + Unpin;
}

#[async_trait(?Send)]
impl CopyFileFromContainer for Vec<u8> {
    type Output = Vec<u8>;

    async fn copy_from_reader<R>(
        mut self,
        reader: R,
    ) -> Result<Self::Output, CopyFromContainerError>
    where
        R: AsyncRead + Unpin,
    {
        let mut_ref = &mut self;
        mut_ref.copy_from_reader(reader).await?;
        Ok(self)
    }
}

#[async_trait(?Send)]
impl<'a> CopyFileFromContainer for &'a mut Vec<u8> {
    type Output = ();

    async fn copy_from_reader<R>(
        mut self,
        mut reader: R,
    ) -> Result<Self::Output, CopyFromContainerError>
    where
        R: AsyncRead + Unpin,
    {
        self.clear();
        reader
            .read_to_end(&mut self)
            .await
            .map_err(CopyFromContainerError::Io)?;
        Ok(())
    }
}

#[async_trait(?Send)]
impl CopyFileFromContainer for PathBuf {
    type Output = ();

    async fn copy_from_reader<R>(self, reader: R) -> Result<Self::Output, CopyFromContainerError>
    where
        R: AsyncRead + Unpin,
    {
        self.as_path().copy_from_reader(reader).await
    }
}

#[async_trait(?Send)]
impl CopyFileFromContainer for &Path {
    type Output = ();

    async fn copy_from_reader<R>(
        self,
        mut reader: R,
    ) -> Result<Self::Output, CopyFromContainerError>
    where
        R: AsyncRead + Unpin,
    {
        if let Some(parent) = self.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(CopyFromContainerError::Io)?;
            }
        }

        let mut file = tokio::fs::File::create(self)
            .await
            .map_err(CopyFromContainerError::Io)?;

        tokio::io::copy(&mut reader, &mut file)
            .await
            .map_err(CopyFromContainerError::Io)?;

        file.flush().await.map_err(CopyFromContainerError::Io)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CopyToContainerError {
    #[error("io failed with error: {0}")]
    IoError(std::io::Error),
    #[error("failed to get the path name: {0}")]
    PathNameError(String),
}

impl CopyToContainerCollection {
    pub fn new(collection: Vec<CopyToContainer>) -> Self {
        Self(collection)
    }

    pub fn add(&mut self, entry: CopyToContainer) {
        self.0.push(entry);
    }

    pub(crate) async fn tar(&self) -> Result<bytes::Bytes, CopyToContainerError> {
        let mut ar = tokio_tar::Builder::new(Vec::new());

        for copy_to_container in &self.0 {
            copy_to_container.append_tar(&mut ar).await?
        }

        let bytes = ar
            .into_inner()
            .await
            .map_err(CopyToContainerError::IoError)?;

        Ok(bytes::Bytes::copy_from_slice(bytes.as_slice()))
    }
}

impl CopyToContainer {
    pub fn new(source: impl Into<CopyDataSource>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
        }
    }

    pub(crate) async fn tar(&self) -> Result<bytes::Bytes, CopyToContainerError> {
        let mut ar = tokio_tar::Builder::new(Vec::new());

        self.append_tar(&mut ar).await?;

        let bytes = ar
            .into_inner()
            .await
            .map_err(CopyToContainerError::IoError)?;

        Ok(bytes::Bytes::copy_from_slice(bytes.as_slice()))
    }

    pub(crate) async fn append_tar(
        &self,
        ar: &mut tokio_tar::Builder<Vec<u8>>,
    ) -> Result<(), CopyToContainerError> {
        self.source.append_tar(ar, &self.target).await
    }
}

impl From<&Path> for CopyDataSource {
    fn from(value: &Path) -> Self {
        CopyDataSource::File(value.to_path_buf())
    }
}

impl From<PathBuf> for CopyDataSource {
    fn from(value: PathBuf) -> Self {
        CopyDataSource::File(value)
    }
}
impl From<Vec<u8>> for CopyDataSource {
    fn from(value: Vec<u8>) -> Self {
        CopyDataSource::Data(value)
    }
}

impl CopyDataSource {
    pub(crate) async fn append_tar(
        &self,
        ar: &mut tokio_tar::Builder<Vec<u8>>,
        target_path: impl Into<String>,
    ) -> Result<(), CopyToContainerError> {
        let target_path: String = target_path.into();

        match self {
            CopyDataSource::File(source_file_path) => {
                if let Err(e) = append_tar_file(ar, source_file_path, &target_path).await {
                    log::error!(
                        "Could not append file/dir to tar: {source_file_path:?}:{target_path}"
                    );
                    return Err(e);
                }
            }
            CopyDataSource::Data(data) => {
                if let Err(e) = append_tar_bytes(ar, data, &target_path).await {
                    log::error!("Could not append data to tar: {target_path}");
                    return Err(e);
                }
            }
        };

        Ok(())
    }
}

async fn append_tar_file(
    ar: &mut tokio_tar::Builder<Vec<u8>>,
    source_file_path: &Path,
    target_path: &str,
) -> Result<(), CopyToContainerError> {
    let target_path = make_path_relative(target_path);
    let meta = tokio::fs::metadata(source_file_path)
        .await
        .map_err(CopyToContainerError::IoError)?;

    if meta.is_dir() {
        ar.append_dir_all(target_path, source_file_path)
            .await
            .map_err(CopyToContainerError::IoError)?;
    } else {
        let f = &mut tokio::fs::File::open(source_file_path)
            .await
            .map_err(CopyToContainerError::IoError)?;

        ar.append_file(target_path, f)
            .await
            .map_err(CopyToContainerError::IoError)?;
    };

    Ok(())
}

async fn append_tar_bytes(
    ar: &mut tokio_tar::Builder<Vec<u8>>,
    data: &Vec<u8>,
    target_path: &str,
) -> Result<(), CopyToContainerError> {
    let relative_target_path = make_path_relative(target_path);

    let mut header = tokio_tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o0644);
    header.set_cksum();

    ar.append_data(&mut header, relative_target_path, data.as_slice())
        .await
        .map_err(CopyToContainerError::IoError)?;

    Ok(())
}

fn make_path_relative(path: &str) -> String {
    // TODO support also absolute windows paths like "C:\temp\foo.txt"
    if path.starts_with("/") {
        path.trim_start_matches("/").to_string()
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write};

    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn copytocontainer_tar_file_success() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "TEST").unwrap();

        let copy_to_container = CopyToContainer::new(file_path, "file.txt");
        let result = copy_to_container.tar().await;

        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
    }

    #[tokio::test]
    async fn copytocontainer_tar_data_success() {
        let data = vec![1, 2, 3, 4, 5];
        let copy_to_container = CopyToContainer::new(data, "data.bin");
        let result = copy_to_container.tar().await;

        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
    }

    #[tokio::test]
    async fn copytocontainer_tar_file_not_found() {
        let temp_dir = tempdir().unwrap();
        let non_existent_file_path = temp_dir.path().join("non_existent_file.txt");

        let copy_to_container = CopyToContainer::new(non_existent_file_path, "file.txt");
        let result = copy_to_container.tar().await;

        assert!(result.is_err());
        if let Err(CopyToContainerError::IoError(err)) = result {
            assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        } else {
            panic!("Expected IoError");
        }
    }

    #[tokio::test]
    async fn copytocontainercollection_tar_file_and_data() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "TEST").unwrap();

        let copy_to_container_collection = CopyToContainerCollection::new(vec![
            CopyToContainer::new(file_path, "file.txt"),
            CopyToContainer::new(vec![1, 2, 3, 4, 5], "data.bin"),
        ]);

        let result = copy_to_container_collection.tar().await;

        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
    }
}
