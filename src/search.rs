mod indexing;
mod parsing;
use indexing::index::DiskIndex;

use std::env;
use std::io::{stdin, stdout, BufRead, BufWriter, Write};

fn main() -> std::io::Result<()> {
    let mut index = DiskIndex::from_disk()?;

    let stdout = stdout();
    let mut out = BufWriter::new(stdout.lock());

    // Parse options
    let args: Vec<String> = env::args().collect();
    let trec = args.iter().any(|a| a == "--trec");

    let stdin = stdin();
    for line in stdin.lock().lines() {
        if let Ok(str) = line {
            if str.is_empty() {
                continue;
            }

            // If we're parsing the query as a TREC query, take the first
            // column to be the query ID.
            let (trec_id, query) = if trec {
                let mut split = str.split_ascii_whitespace();
                let id = split.next().unwrap();

                (
                    Some(id.parse::<u32>().unwrap()),
                    split.fold(String::new(), |mut a, b| {
                        a.push_str(b);
                        a.push(' ');
                        a
                    }),
                )
            } else {
                (None, str)
            };

            let results = index.search(&query)?;
            if let Some(trec_id) = trec_id {
                for r in results.into_iter() {
                    writeln!(
                        out,
                        "{} Q0 {} 0 {} thomas-passmore",
                        trec_id,
                        index.document(r.1),
                        r.0
                    )?;
                }
            } else {
                for r in results.into_iter() {
                    writeln!(out, "{} {}", index.document(r.1), r.0)?;
                }
            }
            out.flush()?;
        }
    }

    Ok(())
}
