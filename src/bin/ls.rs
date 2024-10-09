use std::cell::LazyCell;
use std::fmt::Display;
use std::fs::Metadata;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use clap::Parser;
use time::macros::format_description;
use time::UtcOffset;
use time::{parsing::Parsed, OffsetDateTime};

const UTC_OFFSET: LazyCell<UtcOffset> =
    LazyCell::new(|| UtcOffset::current_local_offset().unwrap());

#[derive(Parser)]
#[command(version, about = "list directory contents", long_about = None)]
struct Cli {
    path: Option<Vec<PathBuf>>,

    /// do not ignore entries starting with .
    #[arg(short, long)]
    all: bool,

    /// use a long listing format
    #[arg(short)]
    long: bool,
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

    fn file_name(&self) -> Option<&std::ffi::OsStr> {
        self.path.file_name()
    }

    fn mode(&self) -> Option<u32> {
        match &self.metadata {
            None => None,
            Some(metadata) => Some(metadata.mode()),
        }
    }
    fn uid(&self) -> Option<u32> {
        match &self.metadata {
            None => None,
            Some(metadata) => Some(metadata.uid()),
        }
    }
    fn gid(&self) -> Option<u32> {
        match &self.metadata {
            None => None,
            Some(metadata) => Some(metadata.gid()),
        }
    }

    fn size(&self) -> Option<u64> {
        match &self.metadata {
            None => None,
            Some(metadata) => Some(metadata.size()),
        }
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

impl<'a> Display for LSFile<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.path.file_name() {
            None => Ok(()),
            Some(file_name) => {
                if self.cli.long {
                    writeln!(
                        f,
                        "{}\t{}\t{}\t{}\t{}\t{}",
                        self.mode().unwrap(),
                        self.uid().unwrap(),
                        self.gid().unwrap(),
                        self.size().unwrap(),
                        self.modified()
                            .unwrap()
                            .format(format_description!(
                                "[month repr:short] [day padding:zero] [hour]:[minute]"
                            ))
                            .unwrap(),
                        file_name.to_string_lossy()
                    )
                } else {
                    write!(f, "{}  ", file_name.to_string_lossy())
                }
            }
        }
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
            return vec![p];
        })
        .filter(|path| {
            return cli.all
                || !path
                    .file_name()
                    .is_some_and(|n| n.as_bytes().starts_with(b"."));
        })
        .map(|p| LSFile::new(p, &cli))
        .collect::<Vec<LSFile>>();
    paths.sort_unstable_by_key(|entry| entry.file_name().unwrap().to_os_string());
    for mut path in paths {
        path.load_metadata();
        print!("{}", path);
    }
    if !cli.long {
        println!("")
    }
}
