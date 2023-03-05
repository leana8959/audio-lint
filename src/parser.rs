use clap::{ArgGroup, Parser};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = "None")]
#[command(group(ArgGroup::new("mode").required(true).multiple(true)))]
pub struct Args {
    #[arg(long, help = "save changes to disk")]
    pub run: bool,

    #[arg(short, long, help = "hush the console output", default_value_t = false)]
    pub quiet: bool,

    #[arg(
        short,
        long = "path",
        help = "provide path to the program",
        required = true
    )]
    pub path: String,

    #[arg(
        short = 't',
        long = "normalize-tracknumber",
        help = "remove padding zeros in track numbers",
        group = "mode"
    )]
    pub normalize_tracknumber: bool,

    #[arg(
        short = 'T',
        long = "normalize-title",
        help = "format title to title case",
        group = "mode"
    )]
    pub normalize_title: bool,

    #[arg(
        short = 'y',
        long = "normalize-year",
        help = "format release year to be four digits",
        group = "mode"
    )]
    pub normalize_year: bool,

    #[arg(
        short,
        long = "rename",
        help = "rename files with metadata",
        group = "mode"
    )]
    pub rename: bool,

    #[arg(
        short,
        long = "clean-others",
        help = "remove comments, lyrics, etc",
        group = "mode"
    )]
    pub clean_others: bool,

    #[arg(
        short = 'g',
        long = "set-genre",
        help = "set genre to",
        group = "mode",
        requires = "genre"
    )]
    pub set_genre: bool,

    #[arg(
        short = 's',
        long = "set-year",
        help = "set year to",
        group = "mode",
        requires = "year"
    )]
    pub set_year: bool,

    #[arg(short = 'G', long = "genre", help = "specify genre", default_value_t = String::from(""))]
    pub genre: String,

    #[arg(short = 'Y', long = "year", help = "specify year", default_value_t = 0)]
    pub year: u32,
}
