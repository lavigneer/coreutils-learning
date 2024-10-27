use lazy_static::lazy_static;
use std::cmp::Ordering;
use std::fmt::Display;
use std::fs::Metadata;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use clap::{ArgAction, Parser};
use coreutils::table::{ColumnAlignment, Table, TableColumn, TableRow};
use humansize::{FormatSizeOptions, BINARY};
use time::macros::format_description;
use time::UtcOffset;
use time::{parsing::Parsed, OffsetDateTime};
use users::{get_group_by_gid, get_user_by_uid};

lazy_static! {
    static ref UTC_OFFSET: UtcOffset = UtcOffset::current_local_offset().unwrap();
}

#[derive(Parser)]
#[command(version, about = "list directory contents", long_about = None, disable_help_flag(true))]
struct Cli {
    path: Option<Vec<PathBuf>>,

    #[arg(short = '?', long, action(ArgAction::Help))]
    help: Option<bool>,

    /// do not ignore entries starting with .
    #[arg(short, long)]
    all: bool,

    /// use a long listing format
    #[arg(short)]
    long: bool,

    /// make the output human readable
    #[arg(short, long)]
    human_readable: bool,

    /// In directories, ignore files that end with ‘~’
    #[arg(short = 'B', long)]
    ignore_backups: bool,

    /// List just the names of directories
    #[arg(short, long)]
    directory: bool,

    /// group directories before files
    #[arg(long)]
    group_directories_first: bool,
}

struct ChMod(u32);

impl Display for ChMod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            format!("{:o}", self.0 & 0o777)
                .chars()
                .map(|c| match c {
                    '1' => "--x",
                    '2' => "-w-",
                    '3' => "-wx",
                    '4' => "r--",
                    '5' => "r-x",
                    '6' => "rw-",
                    '7' => "rwx",
                    _ => "---",
                })
                .collect::<Vec<&str>>()
                .join("")
        )
    }
}

struct LSFile<'a> {
    path: PathBuf,
    cli: &'a Cli,
    metadata: Option<Metadata>,
}

impl<'a> LSFile<'a> {
    fn new(path: PathBuf, cli: &'a Cli) -> Self {
        LSFile {
            path,
            cli,
            metadata: None,
        }
    }

    fn load_metadata(&mut self) {
        self.metadata = self.path.metadata().ok()
    }

    fn is_dir(&self) -> bool {
        self.path.is_dir()
    }

    fn file_name(&self) -> String {
        self.path
            .file_name()
            .map_or("".to_string(), |f| f.to_string_lossy().to_string())
    }

    fn mode(&self) -> Option<ChMod> {
        self.metadata
            .as_ref()
            .map(|metadata| ChMod(metadata.mode()))
    }

    fn uid(&self) -> Option<u32> {
        self.metadata.as_ref().map(|metadata| metadata.uid())
    }

    fn gid(&self) -> Option<u32> {
        self.metadata.as_ref().map(|metadata| metadata.gid())
    }

    fn nlink(&self) -> Option<u64> {
        self.metadata.as_ref().map(|metadata| metadata.nlink())
    }

    fn size(&self) -> Option<u64> {
        self.metadata.as_ref().map(|metadata| metadata.size())
    }

    fn modified(&self) -> Option<OffsetDateTime> {
        match &self.metadata {
            None => None,
            Some(metadata) => {
                let mut parsed_modified = Parsed::new();
                parsed_modified = parsed_modified
                    .with_unix_timestamp_nanos(
                        metadata
                            .modified()
                            .unwrap()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_nanos()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap();
                Some(
                    OffsetDateTime::try_from(parsed_modified)
                        .unwrap()
                        .to_offset(*UTC_OFFSET),
                )
            }
        }
    }
}

impl<'a> Eq for LSFile<'a> {}

impl<'a> PartialEq for LSFile<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl<'a> PartialOrd for LSFile<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for LSFile<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.cli.group_directories_first {
            if self.is_dir() && !other.is_dir() {
                return Ordering::Less;
            } else if other.is_dir() && !self.is_dir() {
                return Ordering::Greater;
            }
        }
        self.file_name().cmp(&other.file_name())
    }
}

impl<'a> From<LSFile<'a>> for TableRow<String, 7> {
    fn from(val: LSFile<'a>) -> Self {
        let size = match val.size() {
            None => "0".to_string(),
            Some(size) => match val.cli.human_readable {
                true => {
                    let custom_options = FormatSizeOptions::from(BINARY)
                        .decimal_places(1)
                        .space_after_value(false)
                        .units(humansize::Kilo::Decimal);
                    let mut size = humansize::format_size(size, custom_options);
                    size.pop();
                    size.to_uppercase()
                }
                false => size.to_string(),
            },
        };
        return TableRow::new([
            format!("{}", val.mode().unwrap()),
            val.nlink().unwrap().to_string(),
            get_user_by_uid(val.uid().unwrap())
                .unwrap()
                .name()
                .to_string_lossy()
                .to_string(),
            get_group_by_gid(val.gid().unwrap())
                .unwrap()
                .name()
                .to_string_lossy()
                .to_string(),
            size,
            val.modified()
                .unwrap()
                .format(format_description!(
                    "[month repr:short] [day padding:zero] [hour]:[minute]"
                ))
                .unwrap(),
            val.file_name(),
        ]);
    }
}

fn main() {
    let cli = Cli::parse();
    let mut paths = cli
        .path
        .clone()
        .unwrap_or(vec![Path::new(".").to_path_buf()])
        .into_iter()
        .flat_map(|p| {
            if p.is_dir() {
                let paths = p
                    .read_dir()
                    .expect("Could not read dir")
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .collect::<Vec<PathBuf>>();
                return paths;
            }
            vec![p]
        })
        .filter(|path| {
            cli.all
                || !path
                    .file_name()
                    .is_some_and(|n| n.as_bytes().starts_with(b"."))
        })
        .filter(|path| !cli.ignore_backups || !path.to_string_lossy().ends_with("~"))
        // TODO: figure out how the ls version works, this doesn't quite match
        .filter(|path| !cli.directory || path.is_dir())
        .map(|p| LSFile::new(p, &cli))
        .collect::<Vec<LSFile>>();
    paths.sort();
    if cli.long {
        let table = Table::new(
            paths
                .into_iter()
                .map(|mut p| {
                    p.load_metadata();
                    p.into()
                })
                .collect::<Vec<TableRow<String, 7>>>(),
            [
                TableColumn::new(ColumnAlignment::Left),
                TableColumn::new(ColumnAlignment::Left),
                TableColumn::new(ColumnAlignment::Left),
                TableColumn::new(ColumnAlignment::Left),
                TableColumn::new(ColumnAlignment::Right),
                TableColumn::new(ColumnAlignment::Left),
                TableColumn::new(ColumnAlignment::Left),
            ],
        );
        print!("{}", table)
    } else {
        for mut path in paths {
            path.load_metadata();
            print!("{} ", path.file_name());
        }
    }
    if !cli.long {
        println!()
    }
}
