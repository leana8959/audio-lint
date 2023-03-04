use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::Parser;
use colored::Colorize;
use metaflac;
use rayon::prelude::IntoParallelRefIterator;
use rayon::prelude::ParallelIterator;
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
fn rename(paths: &mut Vec<PathBuf>, run: bool) -> Vec<String> {
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

        let new_name = format!("{:0>2} - {}.{}", tracknumber, title, ext);
        let new_path = parent.join(&new_name);

        if run {
            let Ok(_) = fs::rename(path, &new_path) else {
                messages.push(format!("{}", old_name.red()));
                continue;
            };
            paths[idx] = new_path.clone();
            messages.push(format!("{} ->\n{}", old_name, new_name.green()));
        } else {
            messages.push(format!("{} ->\n{}", old_name.yellow(), new_name.yellow()));
        }
    }

    return messages;
}

/// Normalize year attibute for a given vector of paths to flac files
fn normalize_year(paths: &Vec<PathBuf>, run: bool) -> Vec<String> {
    paths
        .par_iter()
        .map(|path| {
            let name = path
                .file_name()
                .expect("Path should be a file")
                .to_str()
                .unwrap();

            let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
                return format!("{}", name.red());
            };
            let comments = tag.vorbis_comments_mut();
            let Some(old_date_vec) = comments.get("DATE") else {
                return format!("{}", name.red());
            };
            let Some(old_date) = old_date_vec.iter().next() else  {
                return format!("{}", name.red());
            };
            let re = Regex::new(r"(\d{4})").unwrap();
            let Some(caps) = re.captures(old_date) else {
                return format!("{}", name.red());
            };
            let new_date = caps
                .get(1)
                .map_or(old_date.clone(), |s| s.as_str().to_owned());

            if !run {
                return format!(
                    "{} :\n{}\t{}",
                    name,
                    old_date,
                    new_date.to_string().yellow()
                );
            }

            let result = format!("{} :\n{}\t{}", name, old_date, new_date.to_string().green());
            comments.set("DATE", vec![new_date]);
            let Ok(_) = tag.save() else {
                return format!("{}", name.red());
            };
            return result;
        })
        .collect::<Vec<String>>()
}

/// Normalize tracknumber attribute for a vector of paths to flac files
fn normalize_tracknumber(paths: &Vec<PathBuf>, run: bool) -> Vec<String> {
    paths
        .par_iter()
        .map(|path| {
            let name = path
                .file_name()
                .expect("Path should be a file")
                .to_str()
                .unwrap();

            let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
                return format!("{}", name.red());
            };
            let comments = tag.vorbis_comments_mut();
            let Some(old_number_vec) = comments.get("TRACKNUMBER") else {
                return format!("{}", name.red());
            };
            let Some(old_number) = old_number_vec.iter().next() else  {
                return format!("{}", name.red());
            };
            let Ok(new_number) = old_number.parse::<u32>() else {
                return format!("{}", name.red());
            };

            if !run {
                return format!(
                    "{} :\n{}\t{}",
                    name,
                    old_number,
                    new_number.to_string().yellow()
                );
            }

            let result = format!(
                "{} :\n{}\t{}",
                name,
                old_number,
                new_number.to_string().green()
            );
            comments.set_track(new_number);
            let Ok(_) = tag.save() else {
                return format!("{}", name.red());
            };
            return result;
        })
        .collect::<Vec<String>>()
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
    let run = args.run;

    let mut messages: Vec<String> = Vec::new();

    if args.normalize_tracknumber {
        messages.append(&mut normalize_tracknumber(&paths, run));
    }

    if args.normalize_year {
        messages.append(&mut normalize_year(&paths, run));
    }

    paths = dbg!(paths);
    if args.rename {
        messages.append(&mut rename(&mut paths, run))
    }
    dbg!(paths);

    if !quiet {
        println!("{}", messages.join("\n"));
    }
}
