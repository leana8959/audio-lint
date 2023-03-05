use std::fs;
use std::path::Path;

use colored::Colorize;
use metaflac;
use regex::Regex;
use titlecase::titlecase;
use unic_normal::StrNormalForm;

pub fn normalize_title(path: &Path, run: bool) -> Option<String> {
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
    let Some(old_title_vec) = comments.get("TITLE") else {
        return Some(format!("{}", name.red()));
    };
    let Some(old_title) = old_title_vec.iter().next() else  {
        return Some(format!("{}", name.red()));
    };

    // Normalize track name
    let new_title = titlecase(old_title);

    // Skip if no changes has to be done
    if old_title.nfd().eq(new_title.nfd()) {
        return None;
    }

    // Dry run
    if !run {
        return Some(format!(
            "{} -> {}",
            old_title.strikethrough(),
            new_title.to_string().yellow()
        ));
    }

    // Save changes
    let result = format!(
        "{} -> {}",
        old_title.strikethrough(),
        new_title.to_string().green()
    );
    comments.set_title(vec![new_title]);
    let Ok(_) = tag.save() else {
        return Some(format!("{}", name.red()));
    };
    return Some(result);
}

pub fn set_genre(path: &Path, genre: &String, run: bool) -> Option<String> {
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
            "{} -> {}",
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

pub fn clean_others(path: &Path, run: bool) -> Option<String> {
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
pub fn rename(path: &Path, run: bool) -> Option<String> {
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

pub fn set_year(path: &Path, year: u32, run: bool) -> Option<String> {
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

pub fn normalize_year(path: &Path, run: bool) -> Option<String> {
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

pub fn normalize_tracknumber(path: &Path, run: bool) -> Option<String> {
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
