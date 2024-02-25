mod parser;
mod process;

use crate::parser::Args;
use crate::process::process_entry;
use clap::Parser;
use colored::Colorize;
use std::ffi;
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    let args = Args::parse();

    println!("started...");

    let messages = WalkDir::new(Path::new(&args.path))
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension() == Some(&ffi::OsString::from("flac")))
        .map(|entry| match process_entry(&entry, &args) {
            Ok(msg) => msg.join("\n"),
            Err(err) => err.to_string().red().to_string(),
        })
        .collect::<Vec<String>>()
        .join("\n");

    println!("done!");

    println!("{}", messages);
}
