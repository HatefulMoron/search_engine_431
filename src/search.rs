mod indexing;
mod parsing;
use indexing::index::DiskIndex;
use parsing::terms::Terms;

use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Cursor, Read};

fn main() -> std::io::Result<()> {
    let mut index = DiskIndex::from_disk()?;

    let stdin = stdin();
    for line in stdin.lock().lines() {
        if let Ok(str) = line {
            println!("query: '{}'", str);
            let mut t = Terms::new(str.as_bytes());

            while let Some(term) = t.next() {
                let postings = index.postings(&term)?;
                println!("got {} postings for '{}'", postings.len(), term);
            }
        }
    }

    //let postings = index.postings(String::from("criminal"))?;
    //println!("got {} postings for word", postings.len());
    //let postings = index.postings(String::from("actions"))?;
    //println!("got {} postings for word", postings.len());
    //let postings = index.postings(String::from("officers"))?;
    //println!("got {} postings for word", postings.len());
    //let postings = index.postings(String::from("failed"))?;
    //println!("got {} postings for word", postings.len());
    //let postings = index.postings(String::from("financial"))?;
    //println!("got {} postings for word", postings.len());
    //let postings = index.postings(String::from("institutions"))?;
    //println!("got {} postings for word", postings.len());

    Ok(())
}
