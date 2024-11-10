use crate::fs::{Fs, Metadata};
use anyhow::Context;
use std::path::Path;

pub(crate) struct CowFs<T> {
    fs: T,
}

impl<T: Fs> Fs for CowFs<T> {
    fn name(&self) -> String {
        format!("CoW({})", self.fs.name())
    }
    fn create_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        self.fs.create_dir_all(path)
    }

    fn metadata(&self, path: &Path) -> anyhow::Result<Metadata> {
        self.fs.metadata(path)
    }

    fn copy(&self, from: &Path, to: &Path) -> anyhow::Result<u64> {
        reflink_copy::reflink(&from, &to)
            .with_context(|| format!("failed to reflink {} to {}", from.display(), to.display()))
            .map(|_| 0)
    }

    fn exists(&self, path: &Path) -> bool {
        self.fs.exists(path)
    }
}
