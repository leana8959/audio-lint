use std::fs;
use std::path::Path;

fn read_files(path: &Path) -> Vec<fs::DirEntry> {
    let mut walked: Vec<fs::DirEntry> = Vec::new();
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        if !entry.metadata().unwrap().is_dir() {
            walked.push(entry);
        } else {
            let mut subs = read_files(entry.path().as_path());
            walked.append(&mut subs);
        }
    }

    walked
}

fn main() {
    let path = Path::new("./test");
    dbg!(read_files(&path));
}
