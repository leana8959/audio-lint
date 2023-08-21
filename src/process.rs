use std::fs;
use std::path::Path;

use crate::parser;
use anyhow::anyhow;
use anyhow::Result;
use colored::Colorize;
use metaflac::block::VorbisComment;
use regex::Regex;
use spinner::SpinnerHandle;
use titlecase::titlecase;
use unic_normal::StrNormalForm;
use walkdir::DirEntry;

const TRACKNUMBER: &str = "TRACKNUMBER";
const TITLE: &str = "TITLE";
const GENRE: &str = "GENRE";
const YEAR: &str = "DATE";
const COMMENT: &str = "COMMENT";
const LYRICS: &str = "LYRICS";

struct BeforeAfter {
    old: String,
    new: String,
}

use Change::*;
enum Change {
    Changed(BeforeAfter),
    Cleared,
    Unchanged,
}

fn create_message(msg: Result<Change>, strategy: &str, file_name: &str, run: bool) -> String {
    match msg {
        Ok(msg) => match msg {
            Unchanged => format!("{strategy}: {}", file_name.dimmed()),
            Cleared => {
                let file_name = if run {
                    file_name.green()
                } else {
                    file_name.yellow()
                };
                format!("{strategy}: {}", file_name.strikethrough())
            }
            Changed(BeforeAfter { old, new }) => {
                let new = if run { new.green() } else { new.yellow() };
                format!(r#"{strategy}: "{new}" (was "{old})""#)
            }
        },

        Err(e) => {
            format!("{strategy} {}: {e}", file_name.red())
        }
    }
}

fn edit_tag<S: Strategy>(
    comments: &mut VorbisComment,
    field: &str,
    strategy: S,
) -> Result<Change, anyhow::Error> {
    let old = comments
        .get(field)
        .and_then(|comments| comments.get(0))
        .ok_or(anyhow!("failed load tag: {}", field))?;

    let new = strategy.transform(old)?;

    if strategy.changed(old, &new) {
        return Ok(Unchanged);
    }

    let msg = BeforeAfter {
        old: old.to_owned(),
        new: new.to_owned(),
    };

    comments.set(field, vec![new]);

    Ok(Changed(msg))
}

fn clear_tag(comments: &mut VorbisComment, field: &str) -> Result<Change, anyhow::Error> {
    let res = match comments.comments.remove_entry(field) {
        Some(_) => Cleared,
        None => Unchanged,
    };

    Ok(res)
}

pub trait Strategy {
    fn transform(&self, old: &str) -> Result<String, anyhow::Error>;
    fn changed(&self, old: &str, new: &str) -> bool;
}

struct FormatNumber;
struct FormatText;
struct FormatYear;
struct SetGenre {
    genre: String,
}
struct SetYear {
    year: u32,
}

impl Strategy for FormatNumber {
    fn transform(&self, old: &str) -> Result<String, anyhow::Error> {
        Ok(old.parse::<u32>()?.to_string())
    }
    fn changed(&self, old: &str, new: &str) -> bool {
        old == new
    }
}

impl Strategy for FormatText {
    fn transform(&self, old: &str) -> Result<String, anyhow::Error> {
        let re = Regex::new(r"\s{2}")?;
        Ok(re.replace_all(titlecase(old).trim(), " ").to_string())
    }
    fn changed(&self, old: &str, new: &str) -> bool {
        old.nfd().eq(new.nfd())
    }
}

impl Strategy for FormatYear {
    fn transform(&self, old: &str) -> Result<String, anyhow::Error> {
        Ok(Regex::new(r"(\d{4})")?
            .captures(old)
            .and_then(|group| group.get(1))
            .map_or(old.to_string(), |s| s.as_str().to_string()))
    }
    fn changed(&self, old: &str, new: &str) -> bool {
        old == new
    }
}

impl Strategy for SetGenre {
    fn transform(&self, _old: &str) -> Result<String, anyhow::Error> {
        Ok(self.genre.to_owned())
    }
    fn changed(&self, old: &str, new: &str) -> bool {
        old == new
    }
}

impl Strategy for SetYear {
    fn transform(&self, _old: &str) -> Result<String, anyhow::Error> {
        Ok(self.year.to_string())
    }
    fn changed(&self, old: &str, new: &str) -> bool {
        old == new
    }
}

fn rename(path: &Path, comments: &mut VorbisComment, run: bool) -> Result<Change, anyhow::Error> {
    let old_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(anyhow!("can't get filename for {:?}", path))?;
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or(anyhow!("can't load file extension for {:?}", path))?;
    let parent = path
        .parent()
        .ok_or(anyhow!("can't find parent path for {:?}", path))?;

    let tracknumber = comments
        .get(TRACKNUMBER)
        .and_then(|field| field.get(0))
        .ok_or(anyhow!("can't load tracknumber for {:?}", path))?;
    let title = comments
        .get(TITLE)
        .and_then(|field| field.get(0))
        .ok_or(anyhow!("can't load title for {:?}", path))?;

    let new_name = format!(
        "{:0>2} - {}.{}",
        tracknumber,
        title.replace([':', '/'], " "),
        ext
    );

    if old_name.nfd().eq(new_name.nfd()) {
        return Ok(Unchanged);
    }

    let result = BeforeAfter {
        old: old_name.to_owned(),
        new: new_name.to_owned(),
    };

    if run {
        let new_path = parent.join(&new_name);
        fs::rename(path, new_path)?;
    }

    Ok(Changed(result))
}

pub fn process_entry(
    entry: &DirEntry,
    args: &parser::Args,
    sp: &SpinnerHandle,
) -> Result<Vec<String>, anyhow::Error> {
    let run = args.run;

    let path = entry.path();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(anyhow!("can't load file {:?}", entry))?;

    sp.update(
        path.to_str()
            .ok_or(anyhow!("couldn't convert path"))?
            .to_string(),
    );

    let mut messages: Vec<String> = Vec::new();

    let mut tag = metaflac::Tag::read_from_path(path)?;
    let comments = tag.vorbis_comments_mut();

    let mut tag_modified = false;

    if args.normalize_tracknumber {
        let msg = edit_tag(comments, TRACKNUMBER, FormatNumber);
        if msg.is_ok() {
            tag_modified = true
        };
        messages.push(create_message(msg, "Norm. Numb.", file_name, run));
    }
    if args.normalize_title {
        let msg = edit_tag(comments, TITLE, FormatText);
        if msg.is_ok() {
            tag_modified = true
        };
        messages.push(create_message(msg, "Norm. Title", file_name, run));
    }
    if args.normalize_year {
        let msg = edit_tag(comments, YEAR, FormatYear);
        if msg.is_ok() {
            tag_modified = true
        };
        messages.push(create_message(msg, "Norm. Year", file_name, run));
    }
    if let Some(genre) = &args.set_genre {
        let genre = genre.to_owned();
        let msg = edit_tag(comments, GENRE, SetGenre { genre });
        if msg.is_ok() {
            tag_modified = true
        };
        messages.push(create_message(msg, "Set Genre", file_name, run));
    }
    if let Some(year) = args.set_year {
        let msg = edit_tag(comments, YEAR, SetYear { year });
        if msg.is_ok() {
            tag_modified = true
        };
        messages.push(create_message(msg, "Set Year", file_name, run));
    }

    // Special modes
    if args.rename {
        let msg = rename(path, comments, run);
        messages.push(create_message(msg, "Rename", file_name, run));
    }
    if args.erase {
        let comment_msg = clear_tag(comments, COMMENT);
        let lyrics_msg = clear_tag(comments, LYRICS);

        if comment_msg.is_ok() || lyrics_msg.is_ok() {
            tag_modified = true;
        }

        messages.push(create_message(comment_msg, "Clr. Comment", file_name, run));
        messages.push(create_message(lyrics_msg, "Clr. Lyrics", file_name, run));
    }

    if run && tag_modified {
        tag.save()?
    }

    Ok(messages)
}
