use rayon::prelude::IntoParallelRefIterator;
use rayon::prelude::ParallelIterator;
use std::ffi;
use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

use colored::Colorize;
use metaflac;
use metaflac::block::VorbisComment;
use regex::Regex;
use spinner::SpinnerHandle;
use std::error;
use titlecase::titlecase;
use unic_normal::StrNormalForm;
use walkdir::DirEntry;

use crate::parser;

pub enum Message {
    Unchanged,
    ActionResult { old: String, new: String },
}
impl Message {
    pub fn to_string(&self, prefix: &str, file_name: &String, run: bool) -> String {
        match self {
            Self::Unchanged => format!("{} (unchanged): {}", prefix, file_name.clone().normal()),
            Self::ActionResult { old, new } => {
                format!(
                    "{}: {} -> {}",
                    prefix,
                    old,
                    if !run { new.yellow() } else { new.green() },
                )
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    FileLoadError(&'static str),
    TagParseError(&'static str),
    TagLoadError(&'static str),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileLoadError(msg) => write!(f, "Failed to load file: {}", msg),
            Self::TagParseError(msg) => write!(f, "Failed to parse: {}", msg),
            Self::TagLoadError(msg) => write!(f, "Failed to load tag: {}", msg),
        }
    }
}
impl error::Error for Error {}

fn normalize_tracknumber(comments: &mut VorbisComment) -> Result<Message, Box<dyn error::Error>> {
    let old_number = comments
        .get("TRACKNUMBER")
        .ok_or(Error::TagLoadError("load tracknumber"))?
        .iter()
        .next()
        .ok_or(Error::TagLoadError("load tracknumber"))?;

    let new_number = old_number.parse::<u32>()?;

    // Return if no changes would be made
    if *old_number == new_number.to_string() {
        return Ok(Message::Unchanged);
    }

    let result = Message::ActionResult {
        old: old_number.to_string(),
        new: new_number.to_string(),
    };
    comments.set_track(new_number);
    return Ok(result);
}

fn normalize_title(comments: &mut VorbisComment) -> Result<Message, Box<dyn error::Error>> {
    let old_title = comments
        .get("TITLE")
        .ok_or(Error::TagLoadError("load title"))?
        .iter()
        .next()
        .ok_or(Error::TagLoadError("load title"))?;

    let new_title = titlecase(old_title);

    // Compare using nfd (faster than nfc)
    if old_title.nfd().eq(new_title.nfd()) {
        return Ok(Message::Unchanged);
    }

    let result = Message::ActionResult {
        old: old_title.to_owned(),
        new: new_title.to_owned(),
    };
    comments.set_title(vec![new_title]);
    return Ok(result);
}

fn normalize_year(comments: &mut VorbisComment) -> Result<Message, Box<dyn error::Error>> {
    let old_date = comments
        .get("DATE")
        .ok_or(Error::TagLoadError("load date"))?
        .iter()
        .next()
        .ok_or(Error::TagLoadError("load date"))?;

    let new_date = Regex::new(r"(\d{4})")?
        .captures(old_date)
        .ok_or(Error::TagParseError("parse into new date"))?
        .get(1)
        .map_or(old_date.clone(), |s| s.as_str().to_string());

    // Return if no changes will be made
    if *old_date == new_date {
        return Ok(Message::Unchanged);
    }

    let result = Message::ActionResult {
        old: old_date.to_owned(),
        new: new_date.to_owned(),
    };
    comments.set("DATE", vec![new_date]);
    return Ok(result);
}

fn set_genre(
    comments: &mut VorbisComment,
    genre: &String,
) -> Result<Message, Box<dyn error::Error>> {
    let old_genre = comments
        .get("GENRE")
        .ok_or(Error::TagLoadError("load genre"))?
        .iter()
        .next()
        .ok_or(Error::TagLoadError("load genre"))?;

    let new_genre = genre;

    // Skip if no changes has to be done
    if old_genre == new_genre {
        return Ok(Message::Unchanged);
    }

    let result = Message::ActionResult {
        old: old_genre.to_owned(),
        new: new_genre.to_owned(),
    };
    comments.set_genre(vec![new_genre]);
    return Ok(result);
}

fn clean_others(comments: &mut VorbisComment) -> Result<Message, Box<dyn error::Error>> {
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
        return Ok(Message::Unchanged);
    }

    let result = Message::ActionResult {
        old: "".to_string(),
        new: "Took out the string".to_string(),
    };
    comments.set("COMMENT", vec![""]);
    comments.set("LYRICS", vec![""]);
    return Ok(result);
}

fn set_year(comments: &mut VorbisComment, year: u32) -> Result<Message, Box<dyn error::Error>> {
    let old_date = comments
        .get("DATE")
        .ok_or(Error::TagLoadError("load date"))?
        .iter()
        .next()
        .ok_or(Error::TagLoadError("load date"))?;

    let new_date = year;

    // Return if no changes will be made
    if *old_date == new_date.to_string() {
        return Ok(Message::Unchanged);
    }
    let result = Message::ActionResult {
        old: old_date.to_owned(),
        new: new_date.to_string(),
    };
    comments.set("DATE", vec![new_date.to_string()]);
    return Ok(result);
}

fn rename(
    path: &Path,
    comments: &mut VorbisComment,
    run: bool,
) -> Result<Message, Box<dyn error::Error>> {
    // Unwrap name, extension, and parent path
    let old_name = path
        .file_name()
        .ok_or(Error::FileLoadError("can't get filename"))?
        .to_str()
        .ok_or(Error::FileLoadError("can't get filename"))?;
    let ext = path
        .extension()
        .ok_or(Error::FileLoadError("file extension not present"))?
        .to_str()
        .ok_or(Error::FileLoadError("can't load file extension"))?;
    let parent = path
        .parent()
        .ok_or(Error::FileLoadError("Parent folder isn't valid"))?;

    let tracknumber = comments
        .get("TRACKNUMBER")
        .ok_or(Error::TagLoadError("can't load tracknumber"))?
        .iter()
        .next()
        .ok_or(Error::TagLoadError("can't load tracknumber"))?;
    let title = comments
        .get("TITLE")
        .ok_or(Error::TagLoadError("can't load title"))?
        .iter()
        .next()
        .ok_or(Error::TagLoadError("can't load title"))?;

    // Create new name
    let new_name = format!(
        "{:0>2} - {}.{}",
        tracknumber,
        title.replace(":", " ").replace("/", " "),
        ext
    );

    // Skip if no changes needs to be done
    if old_name.nfd().eq(new_name.nfd()) {
        return Ok(Message::Unchanged);
    }

    let result = Message::ActionResult {
        old: old_name.to_owned(),
        new: new_name.to_owned(),
    };
    if run {
        let new_path = parent.join(&new_name);
        fs::rename(&path, &new_path).unwrap();
    }
    return Ok(result);
}

fn worker(
    entry: &DirEntry,
    args: &parser::Args,
    sp: &Arc<Mutex<SpinnerHandle>>,
) -> Result<Vec<String>, Box<dyn error::Error>> {
    let run = args.run;

    let path = entry.path();
    let file_name = path
        .file_name()
        .ok_or(Error::FileLoadError("filename"))?
        .to_str()
        .ok_or(Error::FileLoadError("filename"))?
        .to_string();

    sp.lock()
        .unwrap()
        .update(path.to_str().unwrap().to_string());

    let mut messages: Vec<String> = Vec::new();

    let mut tag = metaflac::Tag::read_from_path(path)?;
    let comments = tag.vorbis_comments_mut();

    let mut tag_modified = false;

    if args.normalize_tracknumber {
        messages.push(normalize_tracknumber(comments)?.to_string("Norm. num", &file_name, run));
        tag_modified = true;
    }
    if args.normalize_title {
        messages.push(normalize_title(comments)?.to_string("Norm. title", &file_name, run));
        tag_modified = true;
    }
    if args.normalize_year {
        messages.push(normalize_year(comments)?.to_string("Norm. year", &file_name, run));
        tag_modified = true;
    }
    if args.set_genre {
        messages.push(set_genre(comments, &args.genre)?.to_string("Set genre", &file_name, run));
        tag_modified = true;
    }
    if args.clean_others {
        messages.push(clean_others(comments)?.to_string("Remove. junk", &file_name, run));
        tag_modified = true;
    }
    if args.set_year {
        messages.push(set_year(comments, args.year)?.to_string("Set year", &file_name, run));
        tag_modified = true;
    }

    if args.rename {
        messages.push(rename(path, comments, run)?.to_string("Rename", &file_name, run));
    }

    if run && tag_modified {
        tag.save()?
    }

    Ok(messages)
}

pub fn run(args: &parser::Args, sp: &Arc<Mutex<SpinnerHandle>>) -> Mutex<Vec<String>> {
    let messages: Mutex<Vec<String>> = Mutex::new(Vec::new());

    let mut entry_iter = WalkDir::new(Path::new(&args.path))
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some(&ffi::OsString::from("flac")))
        .peekable();

    while entry_iter.peek().is_some() {
        entry_iter
            .by_ref()
            .take(10) // TODO: add thread param
            .collect::<Vec<_>>()
            .par_iter()
            .for_each(|entry| {
                let message = match worker(entry, &args, &sp) {
                    Ok(msg) => msg.join("\n"),
                    Err(err) => err.to_string().red().to_string(),
                };

                messages.lock().unwrap().push(message);
            })
    }

    messages
}
