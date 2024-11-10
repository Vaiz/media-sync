use super::Fs;
use super::{Metadata, ReadonlyFs};
use anyhow::{bail, Context};
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(crate) type ObjectMap = HashMap<PathBuf, (Metadata, Option<PathBuf>)>;

pub(crate) struct DryFs<T> {
    fs: T,
    objects: RefCell<ObjectMap>,
}

impl<T> DryFs<T> {
    pub(crate) fn new(fs: T, objects: RefCell<ObjectMap>) -> Self {
        Self { fs, objects }
    }

    fn add_object(&self, path: PathBuf, meta: Metadata, source: Option<PathBuf>) {
        self.objects.borrow_mut().insert(path, (meta, source));
    }

    fn find_object(&self, path: &Path) -> Option<Ref<Metadata>> {
        let borrow = self.objects.borrow();
        Ref::filter_map(borrow, |objects| objects.get(path).map(|item| &item.0)).ok()
    }
}
impl<T: ReadonlyFs> Fs for DryFs<T> {
    fn name(&self) -> String {
        format!("Dry({})", self.fs.name())
    }
    fn create_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        if Fs::exists(self, path) {
            return Ok(());
        }

        let parent = path
            .parent()
            .with_context(|| format!("Cannot get parent path from [{}]", path.display()))?;
        self.create_dir_all(parent)?;
        self.add_object(path.to_path_buf(), Metadata::dummy_folder(), None);
        Ok(())
    }

    fn metadata(&self, path: &Path) -> anyhow::Result<Metadata> {
        if let Some(metadata) = self.find_object(path) {
            Ok(metadata.clone())
        } else {
            self.fs.metadata(path)
        }
    }

    fn copy(&self, from: &Path, to: &Path) -> anyhow::Result<u64> {
        if Fs::exists(self, to) {
            bail!("Object [{}] already exist", to.display());
        }
        let meta = Fs::metadata(self, from)?;
        let len = meta.len();
        self.add_object(to.to_path_buf(), meta, Some(from.to_path_buf()));
        Ok(len)
    }

    fn exists(&self, path: &Path) -> bool {
        self.find_object(path).is_some() || self.fs.exists(path)
    }
}
