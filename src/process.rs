use std::fs;
use std::path::Path;

use colored::Colorize;
use metaflac;
use metaflac::block::VorbisComment;
use regex::Regex;
use titlecase::titlecase;
use unic_normal::StrNormalForm;

pub enum ProcessResult {
    Nothing,
    ActionResult { old: String, new: String },
}

impl ProcessResult {
    pub fn to_string(&self, prefix: &str, file_name: &String, run: bool) -> String {
        match self {
            Self::Nothing => format!("{} (unchanged): {}", prefix, file_name.clone().normal()),
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

pub fn normalize_tracknumber(comments: &mut VorbisComment) -> Option<ProcessResult> {
    let old_number = comments.get("TRACKNUMBER")?.iter().next()?;

    let new_number = old_number.parse::<u32>().ok()?;

    // Return if no changes would be made
    if *old_number == new_number.to_string() {
        return Some(ProcessResult::Nothing);
    }

    let result = ProcessResult::ActionResult {
        old: old_number.to_string(),
        new: new_number.to_string(),
    };
    comments.set_track(new_number);
    return Some(result);
}

pub fn normalize_title(comments: &mut VorbisComment) -> Option<ProcessResult> {
    let old_title = comments.get("TITLE")?.iter().next()?;

    let new_title = titlecase(old_title);

    // Compare using nfd (faster than nfc)
    if old_title.nfd().eq(new_title.nfd()) {
        return Some(ProcessResult::Nothing);
    }

    let result = ProcessResult::ActionResult {
        old: old_title.to_owned(),
        new: new_title.to_owned(),
    };
    comments.set_title(vec![new_title]);
    return Some(result);
}

pub fn normalize_year(comments: &mut VorbisComment) -> Option<ProcessResult> {
    let old_date = comments.get("DATE")?.iter().next()?;

    let new_date = Regex::new(r"(\d{4})")
        .ok()?
        .captures(old_date)?
        .get(1)
        .map_or(old_date.clone(), |s| s.as_str().to_string());

    // Return if no changes will be made
    if *old_date == new_date {
        return Some(ProcessResult::Nothing);
    }

    let result = ProcessResult::ActionResult {
        old: old_date.to_owned(),
        new: new_date.to_owned(),
    };
    comments.set("DATE", vec![new_date]);
    return Some(result);
}

pub fn set_genre(comments: &mut VorbisComment, genre: &String) -> Option<ProcessResult> {
    let old_genre = comments.get("GENRE")?.iter().next()?;

    let new_genre = genre;

    // Skip if no changes has to be done
    if old_genre == new_genre {
        return Some(ProcessResult::Nothing);
    }

    let result = ProcessResult::ActionResult {
        old: old_genre.to_owned(),
        new: new_genre.to_owned(),
    };
    comments.set_genre(vec![new_genre]);
    return Some(result);
}

pub fn clean_others(comments: &mut VorbisComment) -> Option<ProcessResult> {
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
        return Some(ProcessResult::Nothing);
    }

    let result = ProcessResult::ActionResult {
        old: "".to_string(),
        new: "Took out the string".to_string(),
    };
    comments.set("COMMENT", vec![""]);
    comments.set("LYRICS", vec![""]);
    return Some(result);
}

pub fn set_year(comments: &mut VorbisComment, year: u32) -> Option<ProcessResult> {
    let old_date = comments.get("DATE")?.iter().next()?;

    let new_date = year;

    // Return if no changes will be made
    if *old_date == new_date.to_string() {
        return Some(ProcessResult::Nothing);
    }
    let result = ProcessResult::ActionResult {
        old: old_date.to_owned(),
        new: new_date.to_string(),
    };
    comments.set("DATE", vec![new_date.to_string()]);
    return Some(result);
}

pub fn rename(path: &Path, comments: &mut VorbisComment, run: bool) -> Option<ProcessResult> {
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

    let tracknumber = comments.get("TRACKNUMBER")?.iter().next()?;
    let title = comments.get("TITLE")?.iter().next()?;

    // Create new name
    let new_name = format!(
        "{:0>2} - {}.{}",
        tracknumber,
        title.replace(":", " ").replace("/", " "),
        ext
    );

    // Skip if no changes needs to be done
    if old_name.nfd().eq(new_name.nfd()) {
        return Some(ProcessResult::Nothing);
    }

    let result = ProcessResult::ActionResult {
        old: old_name.to_owned(),
        new: new_name.to_owned(),
    };
    if run {
        let new_path = parent.join(&new_name);
        fs::rename(&path, &new_path).unwrap();
    }
    return Some(result);
}
