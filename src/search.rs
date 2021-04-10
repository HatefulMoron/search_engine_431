mod indexing;
use indexing::index::read_dictionary;
use std::fs::File;
use std::io::BufReader;

fn main() {
    let mut dict = Vec::with_capacity(4192);
    let mut file = File::open("dict.bin").unwrap();
    let mut reader = BufReader::new(file);

    read_dictionary(&mut dict, &mut reader);

    //println!("dict size: {}", dict.len());
}
