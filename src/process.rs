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

enum Message {
    None,
    Some { old: String, new: String },
}
impl Message {
    fn to_string(&self, prefix: &str, file_name: &String, run: bool) -> String {
        match self {
            Self::None => format!("{} (unchanged): {}", prefix, file_name.clone().normal()),
            Self::Some { old, new } => {
                format!(
                    "{}: {} {} -> {}",
                    prefix,
                    file_name,
                    old,
                    if !run { new.yellow() } else { new.green() },
                )
            }
        }
    }
}

#[derive(Debug)]
enum Error {
    FileLoadError(String),
    TagParseError(String),
    TagLoadError(String),
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

fn modify_tag<F, G>(
    comments: &mut VorbisComment,
    field_name: &'static str,
    transform: F,
    is_different: G,
) -> Result<Message, Box<dyn error::Error>>
where
    F: Fn(&String) -> Result<String, Box<dyn error::Error>>,
    G: Fn(&String, &String) -> bool,
{
    let old = comments
        .get(&field_name)
        .ok_or(Error::TagLoadError(format!("load {}", field_name)))?
        .iter()
        .next()
        .ok_or(Error::TagLoadError(format!("load {}", field_name)))?;

    let new = transform(&old)?;

    if is_different(&old, &new) {
        return Ok(Message::None);
    }

    let msg = Message::Some {
        old: old.to_owned(),
        new: new.to_owned(),
    };

    comments.set(field_name, vec![new]);

    Ok(msg)
}

fn normalize_tracknumber(comments: &mut VorbisComment) -> Result<Message, Box<dyn error::Error>> {
    modify_tag(
        comments,
        "TRACKNUMBER",
        |old| Ok(old.parse::<u32>()?.to_string()),
        |old, new| old == new,
    )
}

fn normalize_title(comments: &mut VorbisComment) -> Result<Message, Box<dyn error::Error>> {
    modify_tag(
        comments,
        "TITLE",
        |old| Ok(titlecase(old)),
        |old, new| old.nfd().eq(new.nfd()),
    )
}

fn normalize_year(comments: &mut VorbisComment) -> Result<Message, Box<dyn error::Error>> {
    modify_tag(
        comments,
        "DATE",
        |old| {
            Ok(Regex::new(r"(\d{4})")?
                .captures(old)
                .ok_or(Error::TagParseError("parse into new date".to_string()))?
                .get(1)
                .map_or(old.clone(), |s| s.as_str().to_string()))
        },
        |old, new| old == new,
    )
}

fn set_genre(
    comments: &mut VorbisComment,
    genre: &String,
) -> Result<Message, Box<dyn error::Error>> {
    modify_tag(
        comments,
        "GENRE",
        |_| Ok(genre.to_owned()),
        |old, new| old == new,
    )
}

fn set_year(comments: &mut VorbisComment, year: u32) -> Result<Message, Box<dyn error::Error>> {
    modify_tag(
        comments,
        "DATE",
        |_| Ok(year.to_string()),
        |old, new| old == new,
    )
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
        return Ok(Message::None);
    }

    let result = Message::Some {
        old: "".to_string(),
        new: "Took out the string".to_string(),
    };
    comments.set("COMMENT", vec![""]);
    comments.set("LYRICS", vec![""]);
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
        .ok_or(Error::FileLoadError("can't get filename".to_string()))?
        .to_str()
        .ok_or(Error::FileLoadError("can't get filename".to_string()))?;
    let ext = path
        .extension()
        .ok_or(Error::FileLoadError(
            "file extension not present".to_string(),
        ))?
        .to_str()
        .ok_or(Error::FileLoadError(
            "can't load file extension".to_string(),
        ))?;
    let parent = path.parent().ok_or(Error::FileLoadError(
        "Parent folder isn't valid".to_string(),
    ))?;

    let tracknumber = comments
        .get("TRACKNUMBER")
        .ok_or(Error::TagLoadError("can't load tracknumber".to_string()))?
        .iter()
        .next()
        .ok_or(Error::TagLoadError("can't load tracknumber".to_string()))?;
    let title = comments
        .get("TITLE")
        .ok_or(Error::TagLoadError("can't load title".to_string()))?
        .iter()
        .next()
        .ok_or(Error::TagLoadError("can't load title".to_string()))?;

    // Create new name
    let new_name = format!(
        "{:0>2} - {}.{}",
        tracknumber,
        title.replace(":", " ").replace("/", " "),
        ext
    );

    // Skip if no changes needs to be done
    if old_name.nfd().eq(new_name.nfd()) {
        return Ok(Message::None);
    }

    let result = Message::Some {
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
        .ok_or(Error::FileLoadError("filename".to_string()))?
        .to_str()
        .ok_or(Error::FileLoadError("filename".to_string()))?
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
