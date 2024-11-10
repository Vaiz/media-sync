# media-sync

A command-line tool to organize a media library by moving media files from a source directory to a target directory, structured by the creation date of each file. This program is designed to help maintain an organized library by sorting files into date-based folders and optionally renaming them.

## Features

- Organize files in based on customizable date-based subfolder and filename patterns.
- The program can be rerun with the same parameters, and any duplicates will be resolved automatically.
- If tool cannot extract creation date from a file, it puts the file into `unrecognized` dir.
- Support for dry-run.


## Installation

1. Install Rust and Cargo if they aren’t already installed.
2. [Install MediaInfo](https://github.com/Vaiz/mediameta/blob/master/mediainfo.md).
3. Clone this repository.
4. Build the project with.
5. The executable will be located in `target/release`.

```bash
git clone --depth 1 https://github.com/Vaiz/media-sync.git
cargo build --release
```

## Usage

```bash
media-organizer <source> <target> [options]
```

### Positional Arguments

- `<source>`: Path to the source directory where media files will be recursively searched.
- `<target>`: Path to the target directory where organized media files will be stored.

### Options

- `--target-dir-pattern <pattern>`: Custom pattern for the target directory structure, based on media creation time. The
pattern must be a valid path (e.g., `%Y/%m/%d`).
    - Default: `%Y/%m/%d`

- `--target-file-pattern <pattern>`: Custom pattern for renaming files based on media creation time. The pattern should
form a valid filename (e.g., `%Y-%m-%dT%H%M%S`).
    - Default: `%Y-%m-%dT%H%M%S`

- `--dry-run`: Simulates the organization process, printing all file operations to the console without moving or copying
files. 
  >**Note**: This mode stores metadata of all copied files in memory for duplicate detection.

- `--unrecognized <folder_name>`: Name of the subfolder in the target directory where unrecognized media files are
  stored. Defaults to `unrecognized`.

### Date Pattern Reference

This program uses `chrono` crate for datetime formatting. More information can be found 
[here](https://docs.rs/chrono/0.4.38/chrono/format/strftime/index.html).

### Examples

#### Organize Files with Default Patterns

```bash
media-sync /path/to/source /path/to/target
```

#### Specify Custom Patterns

```bash
media-sync /path/to/source /path/to/target --target-dir-pattern "%Y/%m" --target-file-pattern "%H%M"
```

This command organizes files by year and month in subdirectories and names them with a time stamp.

#### Dry Run

```bash
media-sync.exe D:\tmp\test_data D:\tmp\sorted --dry-run --target-dir-pattern %Y`
```
```
Dry run results:
D:\tmp\sorted\
D:\tmp\sorted\2014\
D:\tmp\sorted\2014\2014-03-09T015545.MP4                       46536726
╰── D:\tmp\test_data\1.MP4
D:\tmp\sorted\2016\
D:\tmp\sorted\2016\2016-10-09T130712.MTS                       31598592
╰── D:\tmp\test_data\00000.MTS
D:\tmp\sorted\2018\
D:\tmp\sorted\2018\2018-08-30T113154.JPG                         902539
╰── D:\tmp\test_data\12.JPG
D:\tmp\sorted\2018\2018-08-30T113218.JPG                        1733635
╰── D:\tmp\test_data\13.JPG
D:\tmp\sorted\2018\2018-08-30T113229.JPG                        2082226
╰── D:\tmp\test_data\11.JPG
D:\tmp\sorted\2018\2018-09-07T160435.JPG                        1259760
╰── D:\tmp\test_data\14.JPG
D:\tmp\sorted\2019\
D:\tmp\sorted\2019\2019-03-13T111520.JPG                        2158680
╰── D:\tmp\test_data\2.JPG
D:\tmp\sorted\2019\2019-03-23T174739.jpg                        2679770
╰── D:\tmp\test_data\3.jpg
D:\tmp\sorted\2019\2019-04-19T135416.JPG                        2933967
╰── D:\tmp\test_data\4.JPG
D:\tmp\sorted\2019\2019-04-19T151220.JPG                        3196211
╰── D:\tmp\test_data\5.JPG
D:\tmp\sorted\2019\2019-04-19T151946.JPG                        3924456
╰── D:\tmp\test_data\6.JPG
D:\tmp\sorted\2019\2019-04-19T153543.JPG                        3432887
╰── D:\tmp\test_data\7.JPG
D:\tmp\sorted\2019\2019-12-13T221834.jpg                        1224990
╰── D:\tmp\test_data\10.jpg
D:\tmp\sorted\2020\
D:\tmp\sorted\2020\2020-03-22T183007.JPG                         376715
╰── D:\tmp\test_data\8.JPG
D:\tmp\sorted\2020\2020-04-12T143314.JPG                        4175983
╰── D:\tmp\test_data\9.JPG
D:\tmp\sorted\2020\2020-04-12T150742.JPG                         996274
╰── D:\tmp\test_data\1.JPG
Copied files: 16
Copied data size: 109213411
```