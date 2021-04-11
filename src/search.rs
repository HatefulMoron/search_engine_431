mod indexing;
mod parsing;
use indexing::index::DiskIndex;
use parsing::terms::Terms;

use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, Cursor, Read, BufWriter, Write};

fn main() -> std::io::Result<()> {
    let mut index = DiskIndex::from_disk()?;

    let stdout = stdout();
    let mut out = BufWriter::new(stdout.lock());

    let stdin = stdin();
    for line in stdin.lock().lines() {
        if let Ok(str) = line {
            let results = index.search(&str)?;
            for r in results {
                writeln!(out, "{} {}", r.0, r.1)?;
            }
            out.flush()?;
        }
    }

    Ok(())
}
