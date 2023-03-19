use std::ffi;
use std::path;
use walkdir::WalkDir;

use crate::parser;

pub fn load_files(args: parser::Args){
    let mut entry_iter = WalkDir::new(path::Path::new(&args.path))
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some(&ffi::OsString::from("flac")))
        .peekable()
}
