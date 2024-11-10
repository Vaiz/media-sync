use crate::fs::{Fs, Metadata};
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

#[derive(Debug, Default)]
pub(crate) struct Stats {
    copied_count: AtomicI64,
    copied_size: AtomicU64,
}

impl Stats {
    fn count_file(&self, size: u64) {
        self.copied_count.fetch_add(1, Ordering::Relaxed);
        self.copied_size.fetch_add(size, Ordering::Relaxed);
    }

    pub(crate) fn copied_count(&self) -> i64 {
        self.copied_count.load(Ordering::Relaxed)
    }
    pub(crate) fn copied_size(&self) -> u64 {
        self.copied_size.load(Ordering::Relaxed)
    }
}

pub(crate) struct StatFs<T> {
    fs: T,
    stats: Rc<Stats>,
}

impl<T> StatFs<T> {
    pub(crate) fn new(fs: T, stats: Rc<Stats>) -> Self {
        Self { fs, stats }
    }

    pub(crate) fn get_underlying_fs(&self) -> &T {
        &self.fs
    }
}

impl<T: Fs> Fs for StatFs<T> {
    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        self.fs.create_dir_all(&path)
    }

    fn metadata<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<Metadata> {
        self.fs.metadata(&path)
    }

    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> anyhow::Result<u64> {
        let size = self.fs.copy(from, to)?;
        self.stats.count_file(size);
        Ok(size)
    }

    fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.fs.exists(path)
    }
}
