use std::{
    io,
    path::{self, Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct CopyToContainer {
    pub target: String,
    pub source: CopyDataSource,
}

#[derive(Debug, Clone)]
pub enum CopyDataSource {
    File(PathBuf),
    Data(Vec<u8>),
}

#[derive(Debug, thiserror::Error)]
pub enum CopyToContaienrError {
    #[error("io failed with error: {0}")]
    IoError(io::Error),
    #[error("failed to get the path name: {0}")]
    PathNameError(String),
}

impl CopyToContainer {
    pub fn target_directory(&self) -> Result<String, CopyToContaienrError> {
        match path::Path::new(&self.target).parent() {
            Some(v) => Ok(v.display().to_string()),
            None => return Err(CopyToContaienrError::PathNameError(self.target.clone())),
        }
    }

    pub async fn tar(&self) -> Result<bytes::Bytes, CopyToContaienrError> {
        self.source.tar(&self.target).await
    }
}

impl CopyDataSource {
    pub async fn tar(
        &self,
        target_path: impl Into<String>,
    ) -> Result<bytes::Bytes, CopyToContaienrError> {
        let target_path: String = target_path.into();
        let mut ar = tokio_tar::Builder::new(Vec::new());

        match self {
            CopyDataSource::File(file_path) => {
                let mut f = &mut tokio::fs::File::open(file_path)
                    .await
                    .map_err(CopyToContaienrError::IoError)?;
                ar.append_file(&target_path, &mut f)
                    .await
                    .map_err(CopyToContaienrError::IoError)?;
            }
            CopyDataSource::Data(data) => {
                let path = path::Path::new(&target_path);
                let file_name = match path.file_name() {
                    Some(v) => v,
                    None => return Err(CopyToContaienrError::PathNameError(target_path)),
                };

                let mut header = tokio_tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o0644);
                header.set_cksum();

                ar.append_data(&mut header, file_name, data.as_slice())
                    .await
                    .map_err(CopyToContaienrError::IoError)?;
            }
        }

        let bytes = ar
            .into_inner()
            .await
            .map_err(CopyToContaienrError::IoError)?;

        Ok(bytes::Bytes::copy_from_slice(bytes.as_slice()))
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
