use std::ffi::OsString;
use std::fs;
use std::path::Path;

use clap::{ArgGroup, Parser};
use colored::Colorize;
use metaflac;
use pager::Pager;
use regex::Regex;
use spinner::SpinnerBuilder;
use unic_normal::StrNormalForm;
use walkdir::WalkDir;

fn set_genre(path: &Path, genre: &String, run: bool) -> Option<String> {
    // Unwrap name
    let name = path
        .file_name()
        .expect("Path should be a file")
        .to_str()
        .unwrap();

    // Read metadata
    let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
        return Some(format!("{}", name.red()));
    };
    let comments = tag.vorbis_comments_mut();
    let Some(old_genre_vec) = comments.get("GENRE") else {
        return Some(format!("{}", name.red()));
    };
    let Some(old_genre) = old_genre_vec.iter().next() else  {
        return Some(format!("{}", name.red()));
    };

    let new_genre = genre;

    // Skip if no changes has to be done
    if old_genre == new_genre {
        return None;
    }

    // Dry run
    if !run {
        return Some(format!(
            "{} ->{}",
            old_genre.strikethrough(),
            new_genre.to_string().yellow()
        ));
    }

    // Save changes
    let result = format!(
        "{} -> {}",
        old_genre.strikethrough(),
        new_genre.to_string().green()
    );
    comments.set_genre(vec![new_genre]);
    let Ok(_) = tag.save() else {
        return Some(format!("{}", name.red()));
    };
    return Some(result);
}

fn clean_others(path: &Path, run: bool) -> Option<String> {
    // Read filename
    let name = path
        .file_name()
        .expect("Path should be a file")
        .to_str()
        .unwrap();

    // Read metadata
    let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
        return Some(format!("{}", name.red()));
    };
    let comments = tag.vorbis_comments_mut();

    let has_comment = comments
        .get("COMMENT")
        .and_then(|v| v.iter().next())
        .map_or(false, |s| !s.is_empty());
    let has_lyrics = comments
        .get("LYRICS")
        .and_then(|v| v.iter().next())
        .map_or(false, |s| !s.is_empty());

    // Return if nothing needs to be done
    if !has_lyrics && !has_comment {
        return None;
    }

    // Dry run
    if !run {
        return Some(format!("{}", name.yellow()));
    }

    // Save changes
    let result = format!("{}", name.green());
    comments.set("COMMENT", vec![""]);
    comments.set("LYRICS", vec![""]);
    let Ok(_) = tag.save() else {
        return Some(format!("{}", name.red()));
    };
    return Some(result);
}

// FIXME
fn rename(path: &Path, run: bool) -> Option<String> {
    // Unwrap name, extension, and parent path
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

    // Read metadata
    let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
        return Some(format!("{}", old_name.red()));
    };
    let comments = tag.vorbis_comments_mut();
    let Some(tracknumber_vec) = comments.get("TRACKNUMBER") else {
        return Some(format!("{}", old_name.red()));
    };
    let Some(tracknumber) = tracknumber_vec.iter().next() else  {
        return Some(format!("{}", old_name.red()));
    };
    let Some(title_vec) = comments.get("TITLE") else {
        return Some(format!("{}", old_name.red()));
    };
    let Some(title) = title_vec.iter().next() else  {
        return Some(format!("{}", old_name.red()));
    };

    // Create new name
    let new_name = format!(
        "{:0>2} - {}.{}",
        tracknumber,
        title.replace(":", " ").replace("/", " "),
        ext
    );

    // Skip if no changes needs to be done
    if old_name.nfd().eq(new_name.nfd()) {
        return None;
    }

    // Dry run
    if !run {
        return Some(format!(
            "{} -> {}",
            old_name.strikethrough(),
            new_name.yellow()
        ));
    }

    // Save changes
    let new_path = parent.join(&new_name);
    let Ok(_) = fs::rename(path, &new_path) else {
        return Some(format!("{}", old_name.red()));
    };
    return Some(format!(
        "{} -> {}",
        old_name.strikethrough(),
        new_name.green()
    ));
}

fn set_year(path: &Path, year: u32, run: bool) -> Option<String> {
    let name = path
        .file_name()
        .expect("Path should be a file")
        .to_str()
        .unwrap();

    // Read year tag
    let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
        return Some(format!("{}", name.red()));
    };
    let comments = tag.vorbis_comments_mut();
    let Some(old_date_vec) = comments.get("DATE") else {
        return Some(format!("{}", name.red()));
    };
    let Some(old_date) = old_date_vec.iter().next() else  {
        return Some(format!("{}", name.red()));
    };

    // Bind new date
    let new_date = year;

    // Return if no changes will be made
    if *old_date == new_date.to_string() {
        return None;
    }

    // Dry run
    if !run {
        return Some(format!(
            "{} -> {}",
            old_date.strikethrough(),
            new_date.to_string().yellow()
        ));
    }

    // Save changes
    let result = format!(
        "{} -> {}",
        old_date.strikethrough(),
        new_date.to_string().green()
    );
    comments.set("DATE", vec![new_date.to_string()]);
    let Ok(_) = tag.save() else {
        return Some(format!("{}", name.red()));
    };
    return Some(result);
}

fn normalize_year(path: &Path, run: bool) -> Option<String> {
    let name = path
        .file_name()
        .expect("Path should be a file")
        .to_str()
        .unwrap();

    // Read year tag
    let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
                return Some(format!("{}", name.red()));
    };
    let comments = tag.vorbis_comments_mut();
    let Some(old_date_vec) = comments.get("DATE") else {
                return Some(format!("{}", name.red()));
    };
    let Some(old_date) = old_date_vec.iter().next() else  {
                return Some(format!("{}", name.red()));
    };

    // Parse into new year
    let re = Regex::new(r"(\d{4})").unwrap();
    let Some(caps) = re.captures(old_date) else {
                return Some(format!("Failed to parse year with regex: {}", name.red()));
    };
    let new_date = caps
        .get(1)
        .map_or(old_date.clone(), |s| s.as_str().to_owned());

    // Return if no changes will be made
    if *old_date == new_date {
        return None;
    }

    // Dry run
    if !run {
        return Some(format!(
            "{} -> {}",
            old_date.strikethrough(),
            new_date.to_string().yellow()
        ));
    }

    // Save changes
    let result = format!(
        "{} -> {}",
        old_date.strikethrough(),
        new_date.to_string().green()
    );
    comments.set("DATE", vec![new_date]);
    let Ok(_) = tag.save() else {
        return Some(format!("{}", name.red()));
    };
    return Some(result);
}

fn normalize_tracknumber(path: &Path, run: bool) -> Option<String> {
    let name = path
        .file_name()
        .expect("Path should be a file")
        .to_str()
        .unwrap();

    // Obtain old number
    let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
        return Some(format!("{}", name.red()));
    };
    let comments = tag.vorbis_comments_mut();
    let Some(old_number_vec) = comments.get("TRACKNUMBER") else {
                return Some(format!("{}", name.red()));
    };
    let Some(old_number) = old_number_vec.iter().next() else  {
        return Some(format!("{}", name.red()));
    };

    // Parse into new number
    let Ok(new_number) = old_number.parse::<u32>() else {
        return Some(format!("Failed to force track number as int {}", name.red()));
    };

    // Return if no changes would be made
    if *old_number == new_number.to_string() {
        return None;
    }

    // Dry run
    if !run {
        return Some(format!(
            "{} -> {}",
            old_number.strikethrough(),
            new_number.to_string().yellow()
        ));
    }

    // Saving the changes
    let result = format!(
        "{} -> {}",
        old_number.strikethrough(),
        new_number.to_string().green()
    );
    comments.set_track(new_number);
    let Ok(_) = tag.save() else {
        return Some(format!("{}", name.red()));
    };
    return Some(result);
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
        long = "set-genre",
        help = "set genre to",
        group = "mode",
        requires = "genre"
    )]
    set_genre: bool,

    #[arg(
        short = 's',
        long = "set-year",
        help = "set year to",
        group = "mode",
        requires = "year"
    )]
    set_year: bool,

    #[arg(short = 'G', long = "genre", help = "specify genre", default_value_t = String::from(""))]
    genre: String,

    #[arg(short = 'Y', long = "year", help = "specify year", default_value_t = 0)]
    year: u32,
}

fn main() {
    let args = Args::parse();
    let quiet = args.quiet;
    let run = args.run;

    let sp = SpinnerBuilder::new("Processing...".to_string())
        .spinner(vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
        .start();

    let mut messages: Vec<String> = Vec::new();

    for entry in WalkDir::new(Path::new(&args.path))
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some(&OsString::from("flac")))
    {
        let path = entry.path();
        sp.update(path.to_str().unwrap().to_string());

        // TODO: remove name feedback in each individual function
        messages.push(format!("{}", path.to_str().unwrap().bold().italic()));
        if args.normalize_tracknumber {
            if let Some(message) = normalize_tracknumber(path, run) {
                messages.push(format!("\tNorm. num.: {}", message));
            }
        }
        if args.normalize_year {
            if let Some(message) = normalize_year(path, run) {
                messages.push(format!("\tNorm. year: {}", message));
            }
        }
        if args.clean_others {
            if let Some(message) = clean_others(path, run) {
                messages.push(format!("\tRemove junk: {}", message));
            }
        }
        if args.set_genre {
            if let Some(message) = set_genre(path, &args.genre, run) {
                messages.push(format!("\tSet genre: {}", message));
            }
        }
        if args.set_year {
            if let Some(message) = set_year(path, args.year, run) {
                messages.push(format!("\tSet year: {}", message));
            }
        }
        if args.rename {
            if let Some(message) = rename(path, run) {
                messages.push(format!("\tRename: {}", message));
            }
        }
    }

    if !quiet {
        if messages.is_empty() {
            sp.message("There's nothing to do, exiting now".to_string());
        } else {
            sp.message("Done !".to_string());
            Pager::with_pager("less -r").setup();
            println!("{}", messages.join("\n"));
        }
    }
}
