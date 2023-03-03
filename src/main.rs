use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::Parser;
use colored::Colorize;
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

/// Rename file based on metadata
fn rename(paths: &mut Vec<PathBuf>) -> Vec<String> {
    let mut messages: Vec<String> = Vec::new();

    for (idx, path) in paths.clone().iter().enumerate() {
        let old_name = path
            .file_name()
            .expect("Path should be a file")
            .to_str()
            .unwrap();
        let ext = path
            .extension()
            .expect("Should have a vaild extension")
            .to_str()
            .unwrap();

        let parent = path.parent().expect("Parent folder should be vaild");

        let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
            messages.push(format!("{}", old_name.red()));
            continue;
        };
        let comments = tag.vorbis_comments_mut();
        let Some(tracknumber_vec) = comments.get("TRACKNUMBER") else {
            messages.push(format!("{}", old_name.red()));
            continue;
        };
        let Some(tracknumber) = tracknumber_vec.iter().next() else  {
            messages.push(format!("{}", old_name.red()));
            continue;
        };
        let Some(title_vec) = comments.get("TITLE") else {
            messages.push(format!("{}", old_name.red()));
            continue;
        };
        let Some(title) = title_vec.iter().next() else  {
            messages.push(format!("{}", old_name.red()));
            continue;
        };

        let new_path = parent.join(format!("{:0>2} - {}.{}", tracknumber, title, ext));

        let Ok(_) = fs::rename(path, &new_path) else {
            messages.push(format!("{}", old_name.red()));
            continue;
        };

        paths[idx] = new_path.clone();
        messages.push(format!("{}", &new_path.to_str().unwrap().green()));
    }

    return messages;
}

/// Normalize year attibute for a given vector of paths to flac files
fn normalize_year(paths: &Vec<PathBuf>) -> Vec<String> {
    let mut messages: Vec<String> = Vec::new();

    for path in paths.iter() {
        let name = path
            .file_name()
            .expect("Path should be a file")
            .to_str()
            .unwrap();

        let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
            messages.push(format!("{}", name.red()));
            continue;
        };
        let comments = tag.vorbis_comments_mut();
        let Some(old_date_vec) = comments.get("DATE") else {
            messages.push(format!("{}", name.red()));
            continue;
        };
        let Some(old_date) = old_date_vec.iter().next() else  {
            messages.push(format!("{}", name.red()));
            continue;
        };
        let re = Regex::new(r"(\d{4})").unwrap();
        let Some(caps) = re.captures(old_date) else {
            messages.push(format!("{}", name.red()));
            continue;
        };
        let new_date = caps
            .get(1)
            .map_or(old_date.clone(), |s| s.as_str().to_owned());
        comments.set("DATE", vec![new_date]);

        let Ok(_) = tag.save() else {
            messages.push(format!("{}", name.red()));
            continue;
        };
        messages.push(format!("{}", name.green()));
    }

    return messages;
}

/// Normalize tracknumber attribute for a vector of paths to flac files
fn normalize_tracknumber(paths: &Vec<PathBuf>) -> Vec<String> {
    let mut messages: Vec<String> = Vec::new();
    for path in paths.iter() {
        let name = path
            .file_name()
            .expect("Path should be a file")
            .to_str()
            .unwrap();

        let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
            messages.push(format!("{}", name.red()));
			continue;
        };
        let comments = tag.vorbis_comments_mut();
        let Some(old_number_vec) = comments.get("TRACKNUMBER") else {
            messages.push(format!("{}", name.red()));
			continue;
        };
        let Some(old_number) = old_number_vec.iter().next() else  {
            messages.push(format!("{}", name.red()));
			continue;
        };
        let Ok(new_number) = old_number.parse::<u32>() else {
            messages.push(format!("{}", name.red()));
			continue;
        };
        comments.set_track(new_number);

        let Ok(_)= tag.save() else {
            messages.push(format!("{}", name.red()));
            continue;
        };
        messages.push(format!("{}", name.green()));
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

    #[arg(short = 'r', long = "rename", help = "rename files with metadata")]
    rename: bool,
}

fn main() {
    let root = Path::new("./test/");
    let mut paths = read_files(root).expect("Please provide a correct path");

    let args = Args::parse();
    let quiet = args.quiet;
    let mut messages: Vec<String> = Vec::new();

    if args.normalize_tracknumber {
        messages.append(&mut normalize_tracknumber(&paths));
    }

    if args.normalize_year {
        messages.append(&mut normalize_year(&paths));
    }

    paths = dbg!(paths);
    if args.rename {
        messages.append(&mut rename(&mut paths))
    }
    dbg!(paths);

    if !quiet {
        println!("{}", messages.join("\n"));
    }
}
