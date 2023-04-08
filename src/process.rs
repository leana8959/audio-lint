use std::fmt;
use std::fs;
use std::num::ParseIntError;
use std::path::Path;

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

const TITLE: &'static str = "TITLE";
const TRACKNUMBER: &'static str = "TRACKNUMBER";
const GENRE: &'static str = "GENRE";
const YEAR: &'static str = "YEAR";
const COMMENT: &'static str = "COMMENT";
const LYRICS: &'static str = "LYRICS";

struct Message {
    old: String,
    new: String,
}

fn format_message(msg: Option<Message>, strategy: &str, file_name: &String, run: bool) -> String {
    match msg {
        None => format!("{} (unchanged): {}", strategy, file_name.clone().normal()),
        Some(Message { old, new }) => {
            format!(
                "{}: {} {} -> {}",
                strategy,
                file_name,
                old,
                if !run { new.yellow() } else { new.green() },
            )
        }
    }
}

#[derive(Debug)]
pub enum EditorError {
    Loadfile(String),
    LoadTag(String),
    ParseTag(String),
}
impl error::Error for EditorError {}

impl From<ParseIntError> for EditorError {
    fn from(value: ParseIntError) -> Self {
        Self::LoadTag(value.to_string())
    }
}
impl From<regex::Error> for EditorError {
    fn from(value: regex::Error) -> Self {
        Self::LoadTag(value.to_string())
    }
}
impl From<metaflac::Error> for EditorError {
    fn from(value: metaflac::Error) -> Self {
        Self::Loadfile(value.to_string())
    }
}

impl fmt::Display for EditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Loadfile(msg) => write!(f, "Failed to load file: {}", msg),
            Self::ParseTag(msg) => write!(f, "Failed to parse: {}", msg),
            Self::LoadTag(msg) => write!(f, "Failed to load tag: {}", msg),
        }
    }
}

struct TagEditor<S> {
    strategy: S,
    field: &'static str,
}

impl<S> TagEditor<S>
where
    S: Strategy,
{
    fn modify(&self, comments: &mut VorbisComment) -> Result<Option<Message>, EditorError> {
        let old = comments
            .get(&self.field)
            .ok_or(EditorError::LoadTag(format!("load {}", self.field)))?
            .iter()
            .next()
            .ok_or(EditorError::LoadTag(format!("load {}", self.field)))?;

        let new = self.strategy.transform(&old)?;

        if self.strategy.changed(&old, &new) {
            return Ok(None);
        }

        let msg = Message {
            old: old.to_owned(),
            new: new.to_owned(),
        };

        comments.set(self.field, vec![new]);

        Ok(Some(msg))
    }
}

trait Strategy {
    fn transform(&self, old: &String) -> Result<String, EditorError>;
    fn changed(&self, old: &String, new: &String) -> bool;
}

struct NormalizeTracknumber;
struct NormalizeTitle;
struct NormalizeYear;
struct Erase;
struct SetGenre {
    genre: String,
}
struct SetYear {
    year: String,
}

impl Strategy for NormalizeTracknumber {
    fn transform(&self, old: &String) -> Result<String, EditorError> {
        Ok(old.parse::<u32>()?.to_string())
    }
    fn changed(&self, old: &String, new: &String) -> bool {
        old == new
    }
}

impl Strategy for NormalizeTitle {
    fn transform(&self, old: &String) -> Result<String, EditorError> {
        Ok(titlecase(old.trim()))
    }
    fn changed(&self, old: &String, new: &String) -> bool {
        old.nfd().eq(new.nfd())
    }
}

impl Strategy for NormalizeYear {
    fn transform(&self, old: &String) -> Result<String, EditorError> {
        Ok(Regex::new(r"(\d{4})")?
            .captures(old)
            .ok_or(EditorError::ParseTag("parse into new date".to_string()))?
            .get(1)
            .map_or(old.clone(), |s| s.as_str().to_string()))
    }
    fn changed(&self, old: &String, new: &String) -> bool {
        old == new
    }
}

impl Strategy for Erase {
    fn transform(&self, _old: &String) -> Result<String, EditorError> {
        Ok("".to_string())
    }
    fn changed(&self, old: &String, _new: &String) -> bool {
        *old != "".to_string()
    }
}

impl Strategy for SetGenre {
    fn transform(&self, _old: &String) -> Result<String, EditorError> {
        Ok(self.genre.to_owned())
    }
    fn changed(&self, old: &String, new: &String) -> bool {
        old == new
    }
}

impl Strategy for SetYear {
    fn transform(&self, _old: &String) -> Result<String, EditorError> {
        Ok(self.year.to_string())
    }
    fn changed(&self, old: &String, new: &String) -> bool {
        old == new
    }
}

fn rename(
    path: &Path,
    comments: &mut VorbisComment,
    run: bool,
) -> Result<Option<Message>, EditorError> {
    let old_name = path
        .file_name()
        .ok_or(EditorError::Loadfile("can't get filename".to_string()))?
        .to_str()
        .ok_or(EditorError::Loadfile("can't get filename".to_string()))?;
    let ext = path
        .extension()
        .ok_or(EditorError::Loadfile(
            "file extension not present".to_string(),
        ))?
        .to_str()
        .ok_or(EditorError::Loadfile(
            "can't load file extension".to_string(),
        ))?;
    let parent = path.parent().ok_or(EditorError::Loadfile(
        "Parent folder isn't valid".to_string(),
    ))?;

    let tracknumber = comments
        .get(TRACKNUMBER)
        .ok_or(EditorError::LoadTag("can't load tracknumber".to_string()))?
        .iter()
        .next()
        .ok_or(EditorError::LoadTag("can't load tracknumber".to_string()))?;
    let title = comments
        .get(TITLE)
        .ok_or(EditorError::LoadTag("can't load title".to_string()))?
        .iter()
        .next()
        .ok_or(EditorError::LoadTag("can't load title".to_string()))?;

    let new_name = format!(
        "{:0>2} - {}.{}",
        tracknumber,
        title.replace(":", " ").replace("/", " "),
        ext
    );

    if old_name.nfd().eq(new_name.nfd()) {
        return Ok(None);
    }

    let result = Message {
        old: old_name.to_owned(),
        new: new_name.to_owned(),
    };

    if run {
        let new_path = parent.join(&new_name);
        fs::rename(&path, &new_path).unwrap();
    }

    return Ok(Some(result));
}

pub fn process_entry(
    entry: &DirEntry,
    args: &parser::Args,
    sp: &SpinnerHandle,
) -> Result<Vec<String>, EditorError> {
    let run = args.run;

    let path = entry.path();
    let file_name = path
        .file_name()
        .ok_or(EditorError::Loadfile("filename".to_string()))?
        .to_str()
        .ok_or(EditorError::Loadfile("filename".to_string()))?
        .to_string();

    sp.update(path.to_str().unwrap().to_string());

    let mut messages: Vec<String> = Vec::new();

    let mut tag = metaflac::Tag::read_from_path(path)?;
    let comments = tag.vorbis_comments_mut();

    let mut tag_modified = false;

    if args.normalize_tracknumber {
        let msg = TagEditor {
            strategy: NormalizeTracknumber,
            field: TRACKNUMBER,
        }
        .modify(comments)?;
        messages.push(format_message(msg, "Norm. Numb.", &file_name, run));
        tag_modified = true;
    }
    if args.normalize_title {
        let msg = TagEditor {
            strategy: NormalizeTitle,
            field: TITLE,
        }
        .modify(comments)?;
        messages.push(format_message(msg, "Norm. Title", &file_name, run));
        tag_modified = true;
    }
    if args.normalize_year {
        let msg = TagEditor {
            strategy: NormalizeYear,
            field: YEAR,
        }
        .modify(comments)?;
        messages.push(format_message(msg, "Norm. Year", &file_name, run));
        tag_modified = true;
    }
    if args.set_genre {
        let msg = TagEditor {
            strategy: SetGenre {
                genre: args.genre.clone(),
            },
            field: GENRE,
        }
        .modify(comments)?;
        messages.push(format_message(msg, "Set Genre", &file_name, run));
        tag_modified = true;
    }
    if args.set_year {
        let msg = TagEditor {
            strategy: SetYear {
                year: args.year.to_string(),
            },
            field: YEAR,
        }
        .modify(comments)?;
        messages.push(format_message(msg, "Set Year", &file_name, run));
        tag_modified = true;
    }

    if args.clean_others {
        let comment_msg = TagEditor {
            strategy: Erase,
            field: COMMENT,
        }
        .modify(comments)?;
        let lyrics_msg = TagEditor {
            strategy: Erase,
            field: LYRICS,
        }
        .modify(comments)?;
        messages.push(format_message(comment_msg, "Rem. Comment", &file_name, run));
        messages.push(format_message(lyrics_msg, "Rem. Lyrics", &file_name, run));
        tag_modified = true;
    }

    if args.rename {
        let msg = rename(path, comments, run)?;
        messages.push(format_message(msg, "Rename", &file_name, run));
    }

    if run && tag_modified {
        tag.save()?
    }

    Ok(messages)
}
