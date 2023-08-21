mod parser;
mod process;

use crate::parser::Args;
use crate::process::process_entry;
use clap::Parser;
use colored::Colorize;
use spinner::SpinnerBuilder;
use std::ffi;
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    let args = Args::parse();

    let sp = SpinnerBuilder::new("Loading files".to_string())
        .spinner(vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
        .start();

    let messages = WalkDir::new(Path::new(&args.path))
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension() == Some(&ffi::OsString::from("flac")))
        .map(|entry| match process_entry(&entry, &args, &sp) {
            Ok(msg) => msg.join("\n"),
            Err(err) => err.to_string().red().to_string(),
        })
        .collect::<Vec<String>>()
        .join("\n");

    sp.message("Done!".to_string());
    sp.close();

    println!("{}", messages);
}
