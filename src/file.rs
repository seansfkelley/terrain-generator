use std::fs;
use std::io::Read;

pub fn read_file_contents<S>(filename: S) -> String where S: Into<String> {
    let fname = filename.into();
    debug!("opening {}", fname);

    let mut contents: String = String::new();
    fs::File::open(fname)
        .expect("couldn't open file")
        .read_to_string(&mut contents)
        .expect("couldn't read file after opening");

    contents
}
