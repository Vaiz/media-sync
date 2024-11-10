pub(crate) mod cow;
pub(crate) mod dry;
pub(crate) mod metadata;
pub(crate) mod stat;

use anyhow::Context;
pub(crate) use metadata::Metadata;
use std::path::Path;

pub(crate) use dry::DryFs;

pub(crate) trait Fs {
    fn name(&self) -> String;
    fn create_dir_all(&self, path: &Path) -> anyhow::Result<()>;
    fn metadata(&self, path: &Path) -> anyhow::Result<Metadata>;
    fn copy(&self, from: &Path, to: &Path) -> anyhow::Result<u64>;
    fn exists(&self, path: &Path) -> bool;
}

pub(crate) trait ReadonlyFs {
    fn name(&self) -> String;
    fn metadata(&self, path: &Path) -> anyhow::Result<Metadata>;
    fn exists(&self, path: &Path) -> bool;
}

impl<T: Fs> ReadonlyFs for T {
    fn name(&self) -> String {
        format!("ReadonlyFs({}", self.name())
    }
    fn metadata(&self, path: &Path) -> anyhow::Result<Metadata> {
        self.metadata(path)
    }

    fn exists(&self, path: &Path) -> bool {
        self.exists(path)
    }
}

#[derive(Default)]
pub(crate) struct StdFs;

impl Fs for StdFs {
    fn name(&self) -> String {
        "StdFs".to_string()
    }

    fn create_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(path)?;
        Ok(())
    }

    fn metadata(&self, path: &Path) -> anyhow::Result<Metadata> {
        Ok(std::fs::metadata(path)?.into())
    }

    fn copy(&self, from: &Path, to: &Path) -> anyhow::Result<u64> {
        Ok(std::fs::copy(from, to)?)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

pub(crate) struct ErrorContextFs<T>(T);

impl<T: Fs> ErrorContextFs<T> {
    pub(crate) fn new(t: T) -> Self {
        Self(t)
    }
}

impl<T: Fs> Fs for ErrorContextFs<T> {
    fn name(&self) -> String {
        format!("ErrorContextFs({})", self.0.name())
    }
    fn create_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        self.0
            .create_dir_all(&path)
            .with_context(|| format!("Failed to create directory [{}]", path.display()))
    }

    fn metadata(&self, path: &Path) -> anyhow::Result<Metadata> {
        self.0
            .metadata(&path)
            .with_context(|| format!("Failed to get metadata of [{}]", path.display()))
    }

    fn copy(&self, from: &Path, to: &Path) -> anyhow::Result<u64> {
        self.0.copy(&from, &to).with_context(|| {
            format!(
                "Failed to copy from [{}] to [{}]",
                from.display(),
                to.display()
            )
        })
    }

    fn exists(&self, path: &Path) -> bool {
        self.0.exists(path)
    }
}
