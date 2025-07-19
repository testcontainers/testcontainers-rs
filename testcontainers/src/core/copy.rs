use std::{
    io,
    path::{Path, PathBuf},
};

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

#[derive(Debug, thiserror::Error)]
pub enum CopyToContainerError {
    #[error("io failed with error: {0}")]
    IoError(io::Error),
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
            assert_eq!(err.kind(), io::ErrorKind::NotFound);
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
