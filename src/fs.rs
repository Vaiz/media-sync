use anyhow::Context;
use std::fs::Metadata;
use std::path::Path;

pub(crate) trait Fs {
    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()>;
    fn metadata<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<Metadata>;
    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> anyhow::Result<u64>;
}

#[derive(Default)]
pub(crate) struct StdFs;

impl Fs for StdFs {
    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        std::fs::create_dir_all(path)?;
        Ok(())
    }

    fn metadata<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<Metadata> {
        Ok(std::fs::metadata(path)?)
    }

    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> anyhow::Result<u64> {
        Ok(std::fs::copy(from, to)?)
    }
}

pub(crate) struct ErrorContextFs<T>(T);

impl<T: Fs> ErrorContextFs<T> {
    pub(crate) fn new(t: T) -> Self {
        Self(t)
    }
}

impl<T: Fs> Fs for ErrorContextFs<T> {
    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        self.0
            .create_dir_all(&path)
            .with_context(|| format!("Failed to create directory [{}]", path.as_ref().display()))
            .into()
    }

    fn metadata<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<Metadata> {
        self.0
            .metadata(&path)
            .with_context(|| format!("Failed to get metadata of [{}]", path.as_ref().display()))
            .into()
    }

    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> anyhow::Result<u64> {
        self.0
            .copy(&from, &to)
            .with_context(|| {
                format!(
                    "Failed to copy from [{}] to [{}]",
                    from.as_ref().display(),
                    to.as_ref().display()
                )
            })
            .into()
    }
}
