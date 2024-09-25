use std::{
    io,
    path::{Path, PathBuf},
};

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

#[derive(Debug, thiserror::Error)]
pub enum CopyToContainerError {
    #[error("io failed with error: {0}")]
    IoError(io::Error),
    #[error("failed to get the path name: {0}")]
    PathNameError(String),
}

impl CopyToContainer {
    pub fn new(source: impl Into<CopyDataSource>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
        }
    }

    pub(crate) async fn tar(&self) -> Result<bytes::Bytes, CopyToContainerError> {
        self.source.tar(&self.target).await
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
    pub(crate) async fn tar(
        &self,
        target_path: impl Into<String>,
    ) -> Result<bytes::Bytes, CopyToContainerError> {
        let target_path: String = target_path.into();

        let bytes = match self {
            CopyDataSource::File(source_file_path) => {
                tar_file(source_file_path, &target_path).await?
            }
            CopyDataSource::Data(data) => tar_bytes(data, &target_path).await?,
        };

        Ok(bytes::Bytes::copy_from_slice(bytes.as_slice()))
    }
}

async fn tar_file(
    source_file_path: &Path,
    target_path: &str,
) -> Result<Vec<u8>, CopyToContainerError> {
    let target_path = make_path_relative(target_path);
    let meta = tokio::fs::metadata(source_file_path)
        .await
        .map_err(CopyToContainerError::IoError)?;

    let mut ar = tokio_tar::Builder::new(Vec::new());
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

    let res = ar
        .into_inner()
        .await
        .map_err(CopyToContainerError::IoError)?;

    Ok(res)
}

async fn tar_bytes(data: &Vec<u8>, target_path: &str) -> Result<Vec<u8>, CopyToContainerError> {
    let relative_target_path = make_path_relative(target_path);

    let mut header = tokio_tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o0644);
    header.set_cksum();

    let mut ar = tokio_tar::Builder::new(Vec::new());
    ar.append_data(&mut header, relative_target_path, data.as_slice())
        .await
        .map_err(CopyToContainerError::IoError)?;

    let res = ar
        .into_inner()
        .await
        .map_err(CopyToContainerError::IoError)?;

    Ok(res)
}

fn make_path_relative(path: &str) -> String {
    // TODO support also absolute windows paths like "C:\temp\foo.txt"
    if path.starts_with("/") {
        path.trim_start_matches("/").to_string()
    } else {
        path.to_string()
    }
}
