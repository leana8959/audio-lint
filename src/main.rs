use std::fs;
use std::io;
use std::path::Path;

fn read_files(path: &Path) -> Result<Vec<fs::DirEntry>, io::Error> {
    let mut walked: Vec<fs::DirEntry> = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if !entry.metadata()?.is_dir() {
            walked.push(entry);
        } else {
            let mut sub_entries = read_files(entry.path().as_path())?;
            walked.append(&mut sub_entries);
        }
    }

    Ok(walked)
}

fn main() {
    let path = Path::new("./test");
    dbg!(read_files(&path));
}
