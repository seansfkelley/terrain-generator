use std::fs;
use std::io::Read;

pub fn read_file_contents(filename: &str) -> String {
    debug!("opening {}", filename);

    let mut contents: String = String::new();
    fs::File::open(filename)
        .expect("couldn't open file")
        .read_to_string(&mut contents)
        .expect("couldn't read file after opening");

    contents
}
