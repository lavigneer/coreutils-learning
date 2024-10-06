use std::fs::DirEntry;
use std::os::unix::fs::MetadataExt;
use std::time::UNIX_EPOCH;
use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use time::macros::format_description;
use time::UtcOffset;
use time::{parsing::Parsed, OffsetDateTime};

#[derive(Parser)]
#[command(version, about = "list directory contents", long_about = None)]
struct Cli {
    path: Option<PathBuf>,

    /// do not ignore entries starting with .
    #[arg(short, long)]
    all: bool,

    /// use a long listing format
    #[arg(short)]
    long: bool,
}

fn main() {
    let cli = Cli::parse();

    let path = if let Some(path) = cli.path.as_deref() {
        path
    } else {
        &Path::new(".")
    };

    let path_metadata = fs::metadata(path);
    let path_metadata = match path_metadata {
        Err(err) => panic!("{}", err),
        Ok(metadata) => metadata,
    };
    if path_metadata.is_file() {
        println!("{}", path.to_path_buf().display());
    } else if path_metadata.is_dir() {
        let mut paths = path
            .read_dir()
            .expect("Could not read dir")
            .filter_map(|entry| entry.ok())
            .collect::<Vec<DirEntry>>();
        paths.sort_by_key(|entry| entry.file_name());
        for entry in paths {
            if let Some(file_name) = entry.file_name().to_str() {
                if cli.all || !file_name.starts_with(".") {
                    if cli.long {
                        let utc_offset = UtcOffset::current_local_offset().unwrap();
                        if let Ok(metadata) = entry.metadata() {
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
                            println!(
                                "{}\t{}\t{}\t{}\t{}",
                                metadata.mode(),
                                metadata.uid(),
                                metadata.gid(),
                                OffsetDateTime::try_from(parsed_modified)
                                    .unwrap()
                                    .to_offset(utc_offset)
                                    .format(format_description!(
                                        "[month repr:short] [day] [hour]:[minute]"
                                    ))
                                    .unwrap(),
                                file_name
                            );
                        }
                    } else {
                        print!("{}  ", entry.file_name().to_string_lossy());
                    }
                }
            }
        }
        if !cli.long {
            println!();
        }
    }

    // println!("ls {}", path.to_path_buf().display())
}
