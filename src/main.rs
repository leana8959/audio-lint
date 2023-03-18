mod parser;
mod process;

use std::ffi::OsString;
use std::path::Path;
use std::sync::{Arc, Mutex};

use clap::Parser;
use colored::Colorize;
use pager::Pager;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use spinner::{SpinnerBuilder, SpinnerHandle};
use walkdir::{DirEntry, WalkDir};

use parser::Args;
use process::*;

fn worker(entry: &DirEntry, args: &Args, sp: &Arc<Mutex<SpinnerHandle>>) -> Option<Vec<String>> {
    let run = args.run;

    let path = entry.path();
    let file_name = path.file_name()?.to_str()?.to_string();

    sp.lock()
        .unwrap()
        .update(path.to_str().unwrap().to_string());

    let mut messages: Vec<String> = Vec::new();

    let mut tag = metaflac::Tag::read_from_path(path).ok()?;
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
        tag.save().ok()?
    }

    Some(messages)
}

fn main() {
    let args = Args::parse();

    let sp = Arc::new(Mutex::new(
        SpinnerBuilder::new("Loading files".to_string())
            .spinner(vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .start(),
    ));

    let messages: Mutex<Vec<String>> = Mutex::new(Vec::new());

    let mut entry_iter = WalkDir::new(Path::new(&args.path))
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some(&OsString::from("flac")))
        .peekable();

    while entry_iter.peek().is_some() {
        entry_iter
            .by_ref()
            .take(10) // TODO: add thread param
            .collect::<Vec<_>>()
            .par_iter()
            .for_each(|entry| {
                let file_name = entry
                    .path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();

                let message = match worker(entry, &args, &sp) {
                    Some(m) => format!("{}\n{}\n", file_name, m.join("\n")),
                    None => format!("Failed: {}\n", file_name.red().to_string()),
                };

                messages.lock().unwrap().push(message);
            });
    }

    // TODO: manage to change the text of the spinner
    println!("\nDone!");

    if !args.quiet {
        Pager::with_pager("less -r").setup();
        println!("{}", messages.lock().unwrap().join("\n"));
    }
}
