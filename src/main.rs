use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use metaflac;

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

fn normalize_year(paths: &Vec<PathBuf>) {
    paths.iter().for_each(|path| {
        let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
            return
        };
        let comments = tag.vorbis_comments_mut();
        let Some(old_date_vec) = comments.get("DATE") else {
            return
        };
        let Some(old_date) = old_date_vec.iter().next() else  {
            return
        };

        let new_date = "test";
        comments.set("DATE", vec![new_date]);

        tag.save().unwrap();
    });
}

fn normalize_tracknumber(paths: &Vec<PathBuf>) {
    paths.iter().for_each(|path| {
        let Ok(mut tag) = metaflac::Tag::read_from_path(path) else {
            return
        };
        let comments = tag.vorbis_comments_mut();
        let Some(old_number_vec) = comments.get("TRACKNUMBER") else {
            return
        };
        let Some(old_number) = old_number_vec.iter().next() else  {
            return
        };
        let Ok(new_number) = old_number.parse::<u32>() else {
            return
        };

        comments.set_track(new_number);

        tag.save().unwrap();
    });
}

fn main() {
    let path = Path::new("./test/");
    let paths = dbg!(read_files(path).unwrap());
    // normalize_tracknumber(&paths);
    normalize_year(&paths);
}
