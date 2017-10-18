use std::{ fs, path };
use std::io::Read;

pub fn read_file_contents(filename: &path::Path) -> String {
    debug!("opening {:?}", filename.to_str());

    let mut contents: String = String::new();
    fs::File::open(&filename)
        .expect("couldn't open file")
        .read_to_string(&mut contents)
        .expect("couldn't read file after opening");

    contents
}
