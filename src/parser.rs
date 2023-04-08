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
        long = "norm-tracknumber",
        help = "remove padding zeros in track numbers",
        group = "mode"
    )]
    pub normalize_tracknumber: bool,

    #[arg(
        short = 'T',
        long = "norm-title",
        help = "format title to title case",
        group = "mode"
    )]
    pub normalize_title: bool,

    #[arg(
        short = 'y',
        long = "norm-year",
        help = "format release year to be four digits",
        group = "mode"
    )]
    pub normalize_year: bool,

    #[arg(
        short = 'r',
        long = "rename",
        help = "rename files with metadata",
        group = "mode"
    )]
    pub rename: bool,

    #[arg(
        short = 'e',
        long = "erase",
        help = "remove comments, lyrics, etc",
        group = "mode"
    )]
    pub erase: bool,

    #[arg(short = 'g', long = "set-genre", help = "set genre to", group = "mode")]
    pub set_genre: bool,

    #[arg(short = 'G', long = "genre", help = "specify genre")]
    pub genre: Option<String>,

    #[arg(long = "set-year", help = "set year to", group = "mode")]
    pub set_year: bool,

    #[arg(long = "year", help = "specify year")]
    pub year: Option<u32>,
}
