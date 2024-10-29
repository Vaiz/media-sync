pub(crate) mod fs;

use anyhow::Context;
use argh::FromArgs;
use chrono::{DateTime, Utc};
use mediameta::extract_file_creation_date;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Organize a media library by creation date, moving media files from source to target directory.
#[derive(FromArgs)]
struct RawArgs {
    /// source path to recursively search for media files.
    #[argh(positional)]
    source: PathBuf,

    /// target path to store organized media files.
    #[argh(positional)]
    target: PathBuf,

    /// subfolder for unrecognized media.
    #[argh(option, default = "\"unrecognized\".to_string()")]
    unrecognized: String,

    /// allows to customize target dir based on media creation time.
    /// The result path should be a set of folders.
    #[argh(option, default = "\"%Y/%m/%d\".to_string()")]
    target_dir_pattern: String,

    /// allows to customize target filename based on media creation time.
    /// The result path should be a valid filename.
    #[argh(option, default = "\"%Y-%m-%dT%H%M%S\".to_string()")]
    target_file_pattern: String,
}

struct Args<T> {
    pub source: PathBuf,
    pub target: PathBuf,
    pub unrecognized: PathBuf,
    pub target_dir_pattern: String,
    pub target_file_pattern: String,
    pub fs: T,
}

impl<T> Args<T> {
    fn new(value: RawArgs, fs: T) -> Self {
        let current_date = Utc::now().format("%Y-%m-%dT%H%M%S").to_string();
        let unrecognized = value.target.join(&value.unrecognized).join(&current_date);
        Self {
            source: value.source,
            target: value.target,
            unrecognized,
            target_dir_pattern: value.target_dir_pattern,
            target_file_pattern: value.target_file_pattern,
            fs,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: RawArgs = argh::from_env();
    let args = Args::new(args, fs::ErrorContextFs::new(fs::StdFs::default()));
    let mut ctx = AppContext::default();
    make_path(&mut ctx, &args, &args.target)?;
    sync_media(&mut ctx, &args)?;
    Ok(())
}

#[derive(Default, Debug)]
struct AppContext {
    created_dirs: std::collections::HashSet<PathBuf>,
}

fn make_path<F: fs::Fs>(ctx: &mut AppContext, args: &Args<F>, path: &Path) -> anyhow::Result<()> {
    if ctx.created_dirs.contains(path) {
        return Ok(());
    }

    args.fs.create_dir_all(path)?;
    ctx.created_dirs.insert(path.to_path_buf());
    Ok(())
}

fn sync_media<F: fs::Fs>(ctx: &mut AppContext, args: &Args<F>) -> anyhow::Result<()> {
    let mut unrecognized_files: Vec<PathBuf> = Vec::new();

    for entry in walkdir::WalkDir::new(&args.source) {
        let entry = entry.with_context(|| "Failed to enumerate source directory")?;
        let path = entry.path();
        if path.is_file() {
            let creation_date = extract_file_creation_date(path);
            if creation_date.is_err() {
                process_unrecognized_file(ctx, &args, path).with_context(|| {
                    format!(
                        "Failed to process unrecognized file [{}]",
                        path.to_string_lossy()
                    )
                })?;
                unrecognized_files.push(path.to_path_buf());
                continue;
            }
            let creation_date: DateTime<Utc> = creation_date.unwrap().into();
            process_file(ctx, args, &path, &args.target, &creation_date)
                .with_context(|| format!("Failed to process file [{}]", path.to_string_lossy()))?;
        }
    }

    // If any unknown files, log their paths
    if !unrecognized_files.is_empty() {
        log_unknown_files(&args, &unrecognized_files)?;
    }

    Ok(())
}

fn process_file<F: fs::Fs>(
    ctx: &mut AppContext,
    args: &Args<F>,
    path: &Path,
    target: &Path,
    creation_date: &DateTime<Utc>,
) -> anyhow::Result<()> {
    let target_subdir = creation_date.format(&args.target_dir_pattern).to_string();
    let target_dir = target.join(target_subdir);
    make_path(ctx, args, &target_dir)?;

    let mut target_filename = creation_date.format(&args.target_file_pattern).to_string();
    if let Some(extension) = path.extension() {
        target_filename = format!("{target_filename}.{}", extension.to_string_lossy())
    }

    copy_file(args, path, &target_dir, &target_filename)?;
    Ok(())
}

fn process_unrecognized_file<F: fs::Fs>(
    ctx: &mut AppContext,
    args: &Args<F>,
    path: &Path,
) -> anyhow::Result<()> {
    let file_name = path
        .file_name()
        .expect("Cannot extract filename")
        .to_string_lossy();
    make_path(ctx, args, &args.unrecognized)?;
    copy_file(args, path, &args.unrecognized, &file_name)
}

fn copy_file<F: fs::Fs>(
    args: &Args<F>,
    source: &Path,
    target_dir: &Path,
    target_filename: &str,
) -> anyhow::Result<()> {
    let source_metadata = args.fs.metadata(source)?;

    let (base_name, extension) = match target_filename.rfind('.') {
        Some(pos) => (&target_filename[..pos], &target_filename[pos..]),
        None => (target_filename, ""),
    };

    let mut target = target_dir.join(target_filename);
    let mut index = 1;
    while target.exists() {
        let target_metadata = args.fs.metadata(&target)?;

        if source_metadata.modified()? == target_metadata.modified()?
            || source_metadata.len() == target_metadata.len()
        {
            println!(
                "Duplicate has been found. Source: [{}], Target: [{}]",
                source.display(),
                target.display()
            );
            return Ok(());
        }

        let new_filename = format!("{base_name}_{index}{extension}");
        target = target_dir.join(new_filename);
        index += 1;
    }

    args.fs.copy(&source, &target)?;
    Ok(())
}

fn log_unknown_files<F>(args: &Args<F>, unknown_files: &Vec<PathBuf>) -> io::Result<()> {
    let log_path = args.unrecognized.join("unknown_files.log");
    let mut log_file = File::create(log_path)?;
    for file in unknown_files {
        writeln!(log_file, "{}", file.display())?;
    }
    Ok(())
}
