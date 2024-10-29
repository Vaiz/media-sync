use argh::FromArgs;
use chrono::{DateTime, Utc};
use mediameta::{extract_combined_metadata};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Organize a media library by creation date, moving media files from source to target directory.
#[derive(FromArgs)]
struct RawArgs {
    /// source path to recursively search for media files
    #[argh(positional)]
    source: PathBuf,

    /// target path to store organized media files
    #[argh(positional)]
    target: PathBuf,

    /// subfolder for unrecognized media
    #[argh(option, default = "\"unrecognized\".to_string()")]
    unrecognized: String,
}

struct Args {
    pub source: PathBuf,
    pub target: PathBuf,
    pub unrecognized: PathBuf,
}

impl From<RawArgs> for Args {
    fn from(value: RawArgs) -> Self {
        let current_date = Utc::now().format("%Y-%m-%dT%H%M%S").to_string();
        let unrecognized = value.target.join(&value.unrecognized).join(&current_date);
        Self {
            source: value.source,
            target: value.target,
            unrecognized,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: RawArgs = argh::from_env();
    let args: Args = args.into();
    let mut ctx = Context::default();
    make_path(&mut ctx, &args.target)?;
    sync_media(&mut ctx, &args)?;
    Ok(())
}

#[derive(Default, Debug)]
struct Context {
    created_dirs: std::collections::HashSet<PathBuf>,
}

fn make_path(ctx: &mut Context, path: &Path) -> anyhow::Result<()> {
    if ctx.created_dirs.contains(path) {
        return Ok(());
    }

    fs::create_dir_all(path)?;
    ctx.created_dirs.insert(path.to_path_buf());
    Ok(())
}

fn sync_media(ctx: &mut Context, args: &Args) -> anyhow::Result<()> {
    let mut unrecognized_files: Vec<PathBuf> = Vec::new();

    for entry in walkdir::WalkDir::new(&args.source) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let metadata = extract_combined_metadata(path);
            if metadata.is_err() || metadata.as_ref().unwrap().creation_date.is_none() {
                process_unrecognized_file(ctx, &args, path)?;
                unrecognized_files.push(path.to_path_buf());
                continue;
            }
            let creation_date = metadata.unwrap().creation_date.unwrap();
            let creation_date : DateTime<Utc> = creation_date.into();
            process_file(ctx, &path, &args.target, &creation_date)?;
        }
    }

    // If any unknown files, log their paths
    if !unrecognized_files.is_empty() {
        log_unknown_files(&args, &unrecognized_files)?;
    }

    Ok(())
}

fn process_file(
    ctx: &mut Context,
    path: &Path,
    target: &Path,
    creation_date: &DateTime<Utc>
) -> anyhow::Result<()> {
    let target_subdir = creation_date.format("%Y/%m/%d").to_string();
    let target_dir = target.join(target_subdir);
    make_path(ctx, &target_dir)?;

    let file_name = format_file_name(path, &target_dir, &creation_date)?;
    let target_file = target_dir.join(file_name);
    copy_or_index_file(path, &target_file)?;
    Ok(())
}

fn process_unrecognized_file(
    ctx: &mut Context,
    args: &Args,
    path: &Path,
) -> anyhow::Result<()> {
    let file_name = path.file_name().expect("Cannot extract filename");
    let target_file = args.unrecognized.join(file_name);
    make_path(ctx, &args.unrecognized)?;
    copy_or_index_file(path, &target_file)
}

fn format_file_name(
    original_path: &Path,
    target_dir: &Path,
    creation_date: &DateTime<Utc>,
) -> io::Result<String> {
    let formatted_name = creation_date.format("%Y-%m-%d %H:%M:%S").to_string();

    let mut unique_name = formatted_name.clone();
    let mut index = 1;
    while target_dir.join(&unique_name).exists() {
        unique_name = format!("{}-{}", formatted_name, index);
        index += 1;
    }
    Ok(unique_name
        + original_path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .as_ref())
}

fn copy_or_index_file(source: &Path, target: &Path) -> anyhow::Result<()> {
    if target.exists() {
        let source_metadata = fs::metadata(source)?;
        let target_metadata = fs::metadata(target)?;

        if source_metadata.modified()? != target_metadata.modified()?
            || source_metadata.len() != target_metadata.len()
        {
            let mut index = 1;
            let mut unique_target = target.to_path_buf();
            while unique_target.exists() {
                unique_target.set_file_name(format!(
                    "{}_{}",
                    target.file_stem().unwrap().to_string_lossy(),
                    index
                ));
                unique_target.set_extension(target.extension().unwrap_or_default());
                index += 1;
            }
            fs::copy(source, unique_target)?;
        }
    } else {
        fs::copy(source, target)?;
    }
    Ok(())
}

fn log_unknown_files(
    args: &Args,
    unknown_files: &Vec<PathBuf>,
) -> io::Result<()> {
    let log_path = args.unrecognized.join("unknown_files.log");
    let mut log_file = File::create(log_path)?;
    for file in unknown_files {
        writeln!(log_file, "{}", file.display())?;
    }
    Ok(())
}
