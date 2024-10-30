use super::Fs;
use super::{Metadata, ReadonlyFs};
use anyhow::{bail, Context};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(crate) struct DryFs<T> {
    fs: T,
    objects: UnsafeCell<HashMap<PathBuf, Metadata>>,
}

impl<T> DryFs<T> {
    pub(crate) fn new(fs: T) -> Self {
        Self {
            fs,
            objects: Default::default(),
        }
    }

    fn add_object(&self, path: PathBuf, meta: Metadata) {
        unsafe { (*self.objects.get()).insert(path, meta) };
    }

    fn find_object<P: AsRef<Path>>(&self, path: P) -> Option<&Metadata> {
        unsafe { (*self.objects.get()).get(path.as_ref()) }
    }

    pub(crate) fn get_map(&self) -> &HashMap<PathBuf, Metadata> {
        unsafe { &*self.objects.get() }
    }
}
impl<T: ReadonlyFs> Fs for DryFs<T> {
    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        if Fs::exists(self, &path) {
            return Ok(());
        }

        let parent = path.as_ref().parent().with_context(|| {
            format!("Cannot get parent path from [{}]", path.as_ref().display())
        })?;
        self.create_dir_all(parent)?;
        self.add_object(path.as_ref().to_path_buf(), Metadata::dummy_folder());
        Ok(())
    }

    fn metadata<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<Metadata> {
        if let Some(metadata) = self.find_object(&path) {
            Ok(metadata.clone())
        } else {
            self.fs.metadata(path)
        }
    }

    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> anyhow::Result<u64> {
        if Fs::exists(self, &to) {
            bail!("Object [{}] already exist", to.as_ref().display());
        }
        let meta = Fs::metadata(self, from)?;
        let len = meta.len();
        self.add_object(to.as_ref().to_path_buf(), meta);
        Ok(len)
    }

    fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.find_object(&path).is_some() || self.fs.exists(path)
    }
}
