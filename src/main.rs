pub(crate) mod fs;

use crate::fs::dry::ObjectMap;
use crate::fs::stat::{StatFs, Stats};
use crate::fs::{Fs, Metadata};
use anyhow::Context;
use argh::FromArgs;
use chrono::{DateTime, Utc};
use mediameta::extract_file_creation_date;
use std::cell::RefCell;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Organize a media library by creation date, moving media files from source to target directory.
#[derive(FromArgs)]
struct RawArgs {
    /// path to the source directory where media files will be recursively searched.
    #[argh(positional)]
    source: String,

    /// path to the target directory where organized media files will be stored.
    #[argh(positional)]
    target: String,

    /// name of the subfolder for unrecognized media files.
    #[argh(option, default = "\"unrecognized\".to_string()")]
    unrecognized: String,

    /// custom pattern for organizing the target directory based on media creation time.
    /// The resulting path will be structured in subfolders.
    /// Default: %Y/%m/%d
    #[argh(option, default = "\"%Y/%m/%d\".to_string()")]
    target_dir_pattern: String,

    /// custom pattern for naming the target file based on media creation time.
    /// The resulting name should be a valid filename.
    /// Default: %Y-%m-%dT%H%M%S
    #[argh(option, default = "\"%Y-%m-%dT%H%M%S\".to_string()")]
    target_file_pattern: String,

    /// simulates the run, outputting all file copy operations without making changes.
    /// WARNING: Stores metadata of all copied files in memory for duplicate detection.
    #[argh(switch)]
    dry_run: bool,
}

struct Args {
    pub source: PathBuf,
    pub target: PathBuf,
    pub unrecognized: PathBuf,
    pub target_dir_pattern: String,
    pub target_file_pattern: String,
    pub dry_run: bool,
    pub fs: Box<dyn Fs>,
}

impl Args {
    fn new(value: RawArgs, fs: Box<dyn Fs>) -> Self {
        let current_date = Utc::now().format("%Y-%m-%dT%H%M%S").to_string();
        let target: PathBuf = Self::fix_separator(&value.target).into();
        let unrecognized = target.join(&value.unrecognized).join(&current_date);
        Self {
            source: Self::fix_separator(&value.source).into(),
            target,
            unrecognized,
            target_dir_pattern: Self::fix_separator(&value.target_dir_pattern),
            target_file_pattern: value.target_file_pattern,
            dry_run: value.dry_run,
            fs,
        }
    }

    fn fix_separator(s: &str) -> String {
        s.replace("\\", std::path::MAIN_SEPARATOR_STR)
            .replace("/", std::path::MAIN_SEPARATOR_STR)
    }
}

fn main() -> anyhow::Result<()> {
    let args: RawArgs = argh::from_env();
    let mut ctx = AppContext::default();

    let stats = Rc::new(Stats::default());
    let mut dry_fs_objects = None;

    let fs: Box<dyn Fs> = if args.dry_run {
        dry_fs_objects = Some(RefCell::new(ObjectMap::new()));
        Box::new(StatFs::new(
            fs::DryFs::new(
                fs::ErrorContextFs::new(fs::StdFs),
                RefCell::clone(dry_fs_objects.as_ref().unwrap()),
            ),
            Rc::clone(&stats),
        ))
    } else {
        Box::new(StatFs::new(
            fs::ErrorContextFs::new(fs::StdFs),
            Rc::clone(&stats),
        ))
    };

    let args = Args::new(args, fs);
    let unrecognized_files = sync_media(&mut ctx, &args)?;

    if args.dry_run {
        println!("Dry run results:");
        print_dry_run(&*dry_fs_objects.unwrap().borrow());
        print_unknown_files(&unrecognized_files);
    } else {
        if !unrecognized_files.is_empty() {
            log_unknown_files(&args, &unrecognized_files)?;
        }
    };

    println!("Copied files: {}", stats.copied_count());
    println!("Copied data size: {}", stats.copied_size());
    Ok(())
}

#[derive(Default, Debug)]
struct AppContext {
    created_dirs: std::collections::HashSet<PathBuf>,
}

fn make_path(ctx: &mut AppContext, args: &Args, path: &Path) -> anyhow::Result<()> {
    if ctx.created_dirs.contains(path) {
        return Ok(());
    }

    args.fs.create_dir_all(path)?;
    ctx.created_dirs.insert(path.to_path_buf());
    Ok(())
}

fn sync_media(ctx: &mut AppContext, args: &Args) -> anyhow::Result<Vec<PathBuf>> {
    let mut unrecognized_files: Vec<PathBuf> = Vec::new();

    make_path(ctx, args, &args.target)?;
    for entry in walkdir::WalkDir::new(&args.source) {
        let entry = entry.with_context(|| "Failed to enumerate source directory")?;
        let path = entry.path();
        if path.is_file() {
            if !can_be_media_file(path) {
                unrecognized_files.push(path.to_path_buf());
                continue;
            }
            let creation_date = extract_file_creation_date(path);
            if creation_date.is_err() {
                process_unrecognized_file(ctx, args, path).with_context(|| {
                    format!("Failed to process the file [{}]", path.to_string_lossy())
                })?;
                unrecognized_files.push(path.to_path_buf());
                continue;
            }
            let creation_date: DateTime<Utc> = creation_date.unwrap().into();
            process_file(ctx, args, path, &args.target, &creation_date)
                .with_context(|| format!("Failed to process file [{}]", path.to_string_lossy()))?;
        }
    }

    Ok(unrecognized_files)
}

fn can_be_media_file(path: &Path) -> bool {
    match path.extension() {
        None => true,
        Some(ext) => !matches!(
            ext.to_string_lossy().to_lowercase().as_str(),
            "bat"
                | "config"
                | "csv"
                | "docx"
                | "exe"
                | "htm"
                | "html"
                | "ini"
                | "json"
                | "log"
                | "md"
                | "pdf"
                | "ppt"
                | "pptx"
                | "rtf"
                | "sh"
                | "txt"
                | "xls"
                | "xlsx"
                | "xml"
                | "yaml"
                | "yml"
        ),
    }
}

fn process_file(
    ctx: &mut AppContext,
    args: &Args,
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

fn process_unrecognized_file(ctx: &mut AppContext, args: &Args, path: &Path) -> anyhow::Result<()> {
    let file_name = path
        .file_name()
        .expect("Cannot extract filename")
        .to_string_lossy();
    make_path(ctx, args, &args.unrecognized)?;
    copy_file(args, path, &args.unrecognized, &file_name)
}

fn copy_file(
    args: &Args,
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
    while args.fs.exists(&target) {
        let target_metadata = args.fs.metadata(&target)?;

        if source_metadata.modified() == target_metadata.modified()
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

    args.fs.copy(source, &target)?;
    Ok(())
}

fn log_unknown_files(args: &Args, unknown_files: &Vec<PathBuf>) -> io::Result<()> {
    let log_path = args.unrecognized.join("unknown_files.log");
    let mut log_file = File::create(log_path)?;
    for file in unknown_files {
        writeln!(log_file, "{}", file.display())?;
    }
    Ok(())
}

fn print_unknown_files(unknown_files: &Vec<PathBuf>) {
    if unknown_files.is_empty() {
        return;
    }
    println!("Unrecognized files:");
    for file in unknown_files {
        println!("{}", file.display());
    }
}

fn print_dry_run(objects: &fs::dry::ObjectMap) {
    let mut sorted: Vec<(&PathBuf, &(Metadata, Option<PathBuf>))> = objects.iter().collect();
    sorted.sort_by(|(path1, _), (path2, _)| path1.cmp(path2));
    for (path, (meta, source)) in sorted {
        if meta.is_dir() {
            println!("{}\\", path.display());
        } else {
            println!("{:<120} {:>10}", path.display(), meta.len());
            if let Some(source) = source {
                println!("╰── {}", source.display())
            }
        }
    }
}
