mod parser;
mod process;

use std::sync::{Arc, Mutex};

use clap::Parser;
use pager::Pager;
use spinner::SpinnerBuilder;

use parser::Args;

fn main() {
    let args = Args::parse();

    let sp = Arc::new(Mutex::new(
        SpinnerBuilder::new("Loading files".to_string())
            .spinner(vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .start(),
    ));

    let messages = process::run(&args, &sp);

    // HACK: manage to change the text of the spinner
    println!("\nDone!");

    if !args.quiet {
        Pager::with_pager("less -r").setup();
        println!("{}", messages.lock().unwrap().join("\n"));
    }
}
