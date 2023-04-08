mod parser;
mod process;

use std::ffi;
use std::path::Path;

use clap::Parser;
use colored::Colorize;
use pager::Pager;
use spinner::SpinnerBuilder;
use walkdir::WalkDir;

use parser::Args;
use process::process_entry;

fn main() {
    let args = Args::parse();

    let sp = SpinnerBuilder::new("Loading files".to_string())
        .spinner(vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
        .start();

    let files_iter = WalkDir::new(Path::new(&args.path))
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some(&ffi::OsString::from("flac")));

    let messages = files_iter
        .map(|entry| match process_entry(&entry, &args, &sp) {
            Ok(msg) => msg.join("\n"),
            Err(err) => err.to_string().red().to_string(),
        })
        .collect::<Vec<String>>()
        .join("\n");

    sp.message("Done!".to_string());
    sp.close();

    if !args.quiet {
        Pager::with_pager("less -r").setup();
        println!("{}", messages);
    }
}
