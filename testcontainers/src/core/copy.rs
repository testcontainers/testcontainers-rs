use std::{
    io,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use tar::EntryType;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct CopyToContainerCollection(Vec<CopyToContainer>);

#[derive(Debug, Clone)]
pub struct CopyToContainer {
    target: CopyTargetOptions,
    source: CopyDataSource,
}

#[derive(Debug, Clone)]
pub struct CopyTargetOptions {
    target: String,
    mode: Option<u32>,
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
    #[error("requested container path is a directory")]
    IsDirectory,
    #[error("archive entry type '{0:?}' is not supported for requested target")]
    UnsupportedEntry(EntryType),
}

/// Abstraction for materializing the bytes read from a source into a concrete destination.
///
/// Implementors typically persist the incoming bytes to disk or buffer them in memory. Some return
/// a value that callers can work with (for example, the collected bytes), while others simply
/// report success with `()`. Implementations must consume the provided reader until EOF or return
/// an error. Destinations are allowed to discard any existing data to make room for the incoming
/// bytes.
#[async_trait(?Send)]
pub trait CopyFileFromContainer {
    type Output;

    /// Writes all bytes from the reader into `self`, returning a value that represents the completed operation (or `()` for sinks that only confirm success).
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
impl CopyFileFromContainer for &mut Vec<u8> {
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
        let mut ar = tar::Builder::new(Vec::new());

        for copy_to_container in &self.0 {
            ar = copy_to_container.append_tar(ar).await?;
        }

        finish_tar(ar)
    }
}

impl CopyToContainer {
    pub fn new(source: impl Into<CopyDataSource>, target: impl Into<CopyTargetOptions>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
        }
    }

    pub(crate) async fn tar(&self) -> Result<bytes::Bytes, CopyToContainerError> {
        let ar = self.append_tar(tar::Builder::new(Vec::new())).await?;

        finish_tar(ar)
    }

    pub(crate) async fn append_tar(
        &self,
        ar: tar::Builder<Vec<u8>>,
    ) -> Result<tar::Builder<Vec<u8>>, CopyToContainerError> {
        self.source.append_tar(ar, &self.target).await
    }
}

fn finish_tar(ar: tar::Builder<Vec<u8>>) -> Result<bytes::Bytes, CopyToContainerError> {
    ar.into_inner()
        .map(bytes::Bytes::from)
        .map_err(CopyToContainerError::IoError)
}

impl CopyTargetOptions {
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            mode: None,
        }
    }

    pub fn with_mode(mut self, mode: u32) -> Self {
        self.mode = Some(mode);
        self
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn mode(&self) -> Option<u32> {
        self.mode
    }
}

impl<T> From<T> for CopyTargetOptions
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        CopyTargetOptions::new(value.into())
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
    /// Appends this source to `ar` under `target`, reading host files on a blocking thread.
    pub(crate) async fn append_tar(
        &self,
        mut ar: tar::Builder<Vec<u8>>,
        target: &CopyTargetOptions,
    ) -> Result<tar::Builder<Vec<u8>>, CopyToContainerError> {
        let target_path = target.target();

        match self {
            CopyDataSource::File(source_file_path) => {
                let source = source_file_path.clone();
                let options = target.clone();
                let appended =
                    run_blocking(move || append_tar_file(&mut ar, &source, &options).map(|()| ar))
                        .await
                        .unwrap_or_else(|e| Err(CopyToContainerError::IoError(e)));
                if appended.is_err() {
                    log::error!(
                        "Could not append file/dir to tar: {source_file_path:?}:{target_path}"
                    );
                }
                appended
            }
            CopyDataSource::Data(data) => {
                if let Err(e) = append_tar_bytes(&mut ar, data, target) {
                    log::error!("Could not append data to tar: {target_path}");
                    return Err(e);
                }
                Ok(ar)
            }
        }
    }
}

/// Runs `f` on tokio's blocking pool and re-raises its panic on the awaiting task.
pub(crate) async fn run_blocking<T, F>(f: F) -> io::Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(value) => Ok(value),
        Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
        Err(error) => Err(io::Error::other(error)),
    }
}

fn append_tar_file(
    ar: &mut tar::Builder<Vec<u8>>,
    source_file_path: &Path,
    target: &CopyTargetOptions,
) -> Result<(), CopyToContainerError> {
    let target_path = make_path_relative(target.target());
    let meta = std::fs::metadata(source_file_path).map_err(CopyToContainerError::IoError)?;

    if meta.is_dir() {
        return ar
            .append_dir_all(target_path, source_file_path)
            .map_err(CopyToContainerError::IoError);
    }

    let file = std::fs::File::open(source_file_path).map_err(CopyToContainerError::IoError)?;

    let mut header = tar::Header::new_gnu();
    header.set_size(meta.len());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = target.mode().unwrap_or_else(|| meta.permissions().mode());
        header.set_mode(mode);
    }

    #[cfg(not(unix))]
    {
        let mode = target.mode().unwrap_or(0o644);
        header.set_mode(mode);
    }

    ar.append_data(&mut header, target_path, file)
        .map_err(CopyToContainerError::IoError)
}

fn append_tar_bytes(
    ar: &mut tar::Builder<Vec<u8>>,
    data: &[u8],
    target: &CopyTargetOptions,
) -> Result<(), CopyToContainerError> {
    let relative_target_path = make_path_relative(target.target());
    let size = u64::try_from(data.len())
        .map_err(|e| CopyToContainerError::IoError(io::Error::other(e)))?;

    let mut header = tar::Header::new_gnu();
    header.set_size(size);
    header.set_mode(target.mode().unwrap_or(0o0644));

    ar.append_data(&mut header, relative_target_path, data)
        .map_err(CopyToContainerError::IoError)
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
    use std::{
        fs::File,
        io::{Read, Write},
    };

    use tar::Archive;
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

    #[tokio::test]
    async fn tar_bytes_respects_custom_mode() {
        let data = vec![1, 2, 3];
        let target = CopyTargetOptions::new("data.bin").with_mode(0o600);
        let copy_to_container = CopyToContainer::new(data, target);

        let tar_bytes = copy_to_container.tar().await.unwrap();
        let mut archive = Archive::new(std::io::Cursor::new(tar_bytes));
        let mut entries = archive.entries().unwrap();
        let entry = entries.next().unwrap().unwrap();

        assert_eq!(entry.header().mode().unwrap(), 0o600);
    }

    #[tokio::test]
    async fn tar_file_respects_custom_mode() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "TEST").unwrap();

        let target = CopyTargetOptions::new("file.txt").with_mode(0o640);
        let copy_to_container = CopyToContainer::new(file_path, target);

        let tar_bytes = copy_to_container.tar().await.unwrap();
        let mut archive = Archive::new(std::io::Cursor::new(tar_bytes));
        let mut entries = archive.entries().unwrap();
        let entry = entries.next().unwrap().unwrap();

        assert_eq!(entry.header().mode().unwrap(), 0o640);
    }

    #[tokio::test]
    async fn tar_directory_copied_to_root_keeps_nested_files() {
        let temp_dir = tempdir().unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        std::fs::write(temp_dir.path().join("subdir/bar.txt"), "bar").unwrap();

        let copy_to_container = CopyToContainer::new(temp_dir.path(), "/");

        let tar_bytes = copy_to_container.tar().await.unwrap();
        let mut archive = Archive::new(std::io::Cursor::new(tar_bytes));
        let mut files = Vec::new();
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            if entry.header().entry_type().is_file() {
                let path = entry.path().unwrap().into_owned();
                let mut contents = String::new();
                entry.read_to_string(&mut contents).unwrap();
                files.push((path, contents));
            }
        }

        assert_eq!(
            files,
            [(PathBuf::from("subdir/bar.txt"), "bar".to_string())]
        );
    }
}
