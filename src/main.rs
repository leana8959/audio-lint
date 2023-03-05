mod parser;
mod process;

use std::ffi::OsString;
use std::path::Path;

use clap::Parser;
use colored::Colorize;
use pager::Pager;
use spinner::SpinnerBuilder;
use walkdir::WalkDir;

use parser::Args;
use process::*;

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
        let mut buffer: Vec<String> = Vec::new();

        if args.normalize_tracknumber {
            if let Some(message) = normalize_tracknumber(path, run) {
                buffer.push(format!("\tNorm. num.: {}", message));
            }
        }
        if args.normalize_title {
            if let Some(message) = normalize_title(path, run) {
                buffer.push(format!("\tNorm. title: {}", message));
            }
        }
        if args.normalize_year {
            if let Some(message) = normalize_year(path, run) {
                buffer.push(format!("\tNorm. year: {}", message));
            }
        }
        if args.clean_others {
            if let Some(message) = clean_others(path, run) {
                buffer.push(format!("\tRemove junk: {}", message));
            }
        }
        if args.set_genre {
            if let Some(message) = set_genre(path, &args.genre, run) {
                buffer.push(format!("\tSet genre: {}", message));
            }
        }
        if args.set_year {
            if let Some(message) = set_year(path, args.year, run) {
                buffer.push(format!("\tSet year: {}", message));
            }
        }
        if args.rename {
            if let Some(message) = rename(path, run) {
                buffer.push(format!("\tRename: {}", message));
            }
        }

        if !buffer.is_empty() {
            messages.push(format!("{}", path.to_str().unwrap().bold().italic()));
            messages.append(&mut buffer);
        }
    }

    sp.message("Done !".to_string());
    println!("\n");

    if !quiet {
        if messages.is_empty() {
            println!("There's nothing to do, exiting now");
        } else {
            Pager::with_pager("less -r").setup();
            println!("{}", messages.join("\n"));
        }
    }
}
