use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::Parser;
use metaflac;
use regex::Regex;

/// Read files from given path, recursively.
fn read_files(path: &Path) -> Result<Vec<PathBuf>, io::Error> {
    fn get_entries(path: &Path) -> Result<Vec<fs::DirEntry>, io::Error> {
        let mut walked: Vec<fs::DirEntry> = Vec::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.metadata()?.is_dir() {
                let mut sub_entries = get_entries(entry.path().as_path())?;
                walked.append(&mut sub_entries);
            } else if entry.path().extension() == Some(&OsString::from("flac")) {
                walked.push(entry);
            }
        }

        Ok(walked)
    }

    get_entries(&path)?
        .iter()
        .map(|entry| Ok(entry.path()))
        .collect()
}

fn normalize_year(paths: &Vec<PathBuf>) -> Vec<String> {
    let mut messages: Vec<String> = Vec::new();

    for path in paths {
        let name = path
            .file_name()
            .expect("Path should be a file")
            .to_str()
            .unwrap();

        let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
            messages.push(format!("Skipped {name}"));
            continue;
        };
        let comments = tag.vorbis_comments_mut();
        let Some(old_date_vec) = comments.get("DATE") else {
            messages.push(format!("Skipped {name}"));
            continue;
        };
        let Some(old_date) = old_date_vec.iter().next() else  {
            messages.push(format!("Skipped {name}"));
            continue;
        };
        let re = Regex::new(r"(\d{4})").unwrap();
        let Some(caps) = re.captures(old_date) else {
            messages.push(format!("Skipped {name}"));
            continue;
        };
        let new_date = caps
            .get(1)
            .map_or(old_date.clone(), |s| s.as_str().to_owned());
        comments.set("DATE", vec![new_date]);

        let Ok(_) = tag.save() else {
            messages.push(format!("Skipped {name}"));
            continue;
        };
        messages.push(format!("Processed {name}"));
    }

    return messages;
}

fn normalize_tracknumber(paths: &Vec<PathBuf>) -> Vec<String> {
    let mut messages: Vec<String> = Vec::new();
    for path in paths {
        let name = path
            .file_name()
            .expect("Path should be a file")
            .to_str()
            .unwrap();

        let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
            messages.push(format!("Skipped {name}"));
			continue;
        };
        let comments = tag.vorbis_comments_mut();
        let Some(old_number_vec) = comments.get("TRACKNUMBER") else {
            messages.push(format!("Skipped {name}"));
			continue;
        };
        let Some(old_number) = old_number_vec.iter().next() else  {
            messages.push(format!("Skipped {name}"));
			continue;
        };
        let Ok(new_number) = old_number.parse::<u32>() else {
            messages.push(format!("Skipped {name}"));
			continue;
        };
        comments.set_track(new_number);

        let Ok(_)= tag.save() else {
            messages.push(format!("Skipped {name}"));
            continue;
        };
        messages.push(format!("Processed {name}"))
    }
    return messages;
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = "None")]
struct Args {
    #[arg(long, help = "save changes to disk")]
    run: bool,

    #[arg(short, long, help = "show what changes will be / has been made")]
    quiet: bool,

    #[arg(
        short = 't',
        long = "normalize-tracknumber",
        help = "remove padding zeros in track numbers"
    )]
    normalize_tracknumber: bool,

    #[arg(
        short = 'y',
        long = "normalize-year",
        help = "format release year to be four digits"
    )]
    normalize_year: bool,
}

fn main() {
    let path = Path::new("./test/");
    let paths = dbg!(read_files(path).unwrap());
    // normalize_tracknumber(&paths);
    // normalize_year(&paths);

    let args = Args::parse();
    let quiet = dbg!(args.quiet);

    if args.normalize_tracknumber {
        dbg!(normalize_tracknumber(&paths));
    }

    if args.normalize_year {
        dbg!(normalize_year(&paths));
    }
}
