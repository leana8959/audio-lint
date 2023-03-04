use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::{ArgGroup, Parser};
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

/// Set/Clear genre
fn set_genre(paths: &Vec<PathBuf>, genre: String, run: bool) -> Vec<String> {
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
            let Some(old_genre_vec) = comments.get("GENRE") else {
                return format!("{}", name.red());
            };
            let Some(old_genre) = old_genre_vec.iter().next() else  {
                return format!("{}", name.red());
            };
            let new_genre = &genre;

            if !run {
                return format!(
                    "{} :\n{}\t{}",
                    name,
                    old_genre,
                    new_genre.to_string().yellow()
                );
            }

            let result = format!(
                "{} :\n{}\t{}",
                name,
                old_genre,
                new_genre.to_string().green()
            );
            comments.set_genre(vec![new_genre]);
            let Ok(_) = tag.save() else {
                return format!("{}", name.red());
            };
            return result;
        })
        .collect::<Vec<String>>()
}

/// Remove redundant informations
fn clean_others(paths: &Vec<PathBuf>, run: bool) -> Vec<String> {
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

            if !run {
                return format!("{}", name.yellow());
            }

            let result = format!("{}", name.green());
            let comments = tag.vorbis_comments_mut();
            comments.set("COMMENT", vec![""]);
            comments.set("LYRICS", vec![""]);
            let Ok(_) = tag.save() else {
                return format!("{}", name.red());
            };
            return result;
        })
        .collect::<Vec<String>>()
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
#[command(group(ArgGroup::new("mode").required(true).multiple(true)))]
struct Args {
    #[arg(long, help = "save changes to disk")]
    run: bool,

    #[arg(short, long, help = "hush the console output", default_value_t = false)]
    quiet: bool,

    #[arg(
        short,
        long = "path",
        help = "provide path to the program",
        required = true
    )]
    path: String,

    #[arg(
        short = 't',
        long = "normalize-tracknumber",
        help = "remove padding zeros in track numbers",
        group = "mode"
    )]
    normalize_tracknumber: bool,

    #[arg(
        short = 'y',
        long = "normalize-year",
        help = "format release year to be four digits",
        group = "mode"
    )]
    normalize_year: bool,

    #[arg(
        short,
        long = "rename",
        help = "rename files with metadata",
        group = "mode"
    )]
    rename: bool,

    #[arg(
        short,
        long = "clean-others",
        help = "remove comments, lyrics, etc",
        group = "mode"
    )]
    clean_others: bool,

    #[arg(
        short = 'g',
        long = "set_genre",
        help = "set genre to",
        group = "mode",
        requires = "genre"
    )]
    set_genre: bool,

    #[arg(short = 'G', long = "genre", help = "specify genre")]
    genre: String,
}

fn main() {
    let args = Args::parse();
    let quiet = args.quiet;
    let run = args.run;
    let genre = args.genre;

    // let root = Path::new("./test/");
    let root = Path::new(&args.path);
    let mut paths = read_files(root).expect("Please provide a valid path");

    let mut messages: Vec<String> = Vec::new();

    if args.normalize_tracknumber {
        messages.append(&mut normalize_tracknumber(&paths, run));
    }

    if args.normalize_year {
        messages.append(&mut normalize_year(&paths, run));
    }

    if args.rename {
        messages.append(&mut rename(&mut paths, run))
    }

    if args.clean_others {
        messages.append(&mut clean_others(&paths, run));
    }

    if args.set_genre {
        messages.append(&mut set_genre(&paths, genre, run));
    }

    if !quiet {
        println!("{}", messages.join("\n"));
    }
}
