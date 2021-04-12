mod indexing;
mod parsing;
use indexing::index::DiskIndex;

use std::io::{stdin, stdout, BufRead, BufWriter, Write};

fn main() -> std::io::Result<()> {
    let mut index = DiskIndex::from_disk()?;

    let stdout = stdout();
    let mut out = BufWriter::new(stdout.lock());

    let stdin = stdin();
    for line in stdin.lock().lines() {
        if let Ok(str) = line {
            let results = index.search(&str)?;
            for r in results.into_iter() {
                writeln!(out, "{} {}", index.document(r.1), r.0)?;
            }
            out.flush()?;
        }
    }

    Ok(())
}
