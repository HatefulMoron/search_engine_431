use std::collections::BTreeMap;
use std::io;
use std::io::{BufWriter, Read, Write};

mod indexing;
mod parsing;
use indexing::index::{write_documents, write_postings, write_term, Document, Posting};
use indexing::varint::write_varint;
use std::fs::File;

fn main() -> std::io::Result<()> {
    let stdin = io::stdin();
    let mut content = String::new();
    stdin.lock().read_to_string(&mut content).unwrap();

    // Docno, term count
    let mut documents: Vec<(&str, u32)> = Vec::new();
    let mut term_count: u32 = 0;

    // Term -> [document -> frequency]
    // Dictionary is set of terms/keys
    let mut index: BTreeMap<&str, BTreeMap<u32, u32>> = BTreeMap::new();
    let mut lines = content.split(|c| c == '\n');

    loop {
        let line = match lines.next() {
            Some(l) => l,
            None => break,
        };

        if documents.is_empty() {
            documents.push((line, 0));
            continue;
        }

        if line.is_empty() {
            match lines.next() {
                Some(doc_line) => {
                    // TODO: refactor
                    let prev = documents.len() - 1;
                    documents[prev].1 = term_count;
                    term_count = 0;
                    documents.push((doc_line, 0))
                }
                None => break,
            }
            continue;
        }

        term_count += 1;

        assert!(!line.is_empty());

        match index.get_mut(line) {
            Some(map) => {
                let k = documents.len() as u32 - 1;
                match map.get_mut(&k) {
                    Some(p) => *p += 1,
                    None => {
                        map.insert(documents.len() as u32 - 1, 1);
                    }
                }
            }
            None => {
                let mut map = BTreeMap::new();
                map.insert(documents.len() as u32 - 1, 1);
                index.insert(line, map);
            }
        };
    }

    // TODO: refactor
    let prev = documents.len() - 1;
    documents[prev].1 = term_count;

    // Write documents
    {
        let avg_dl = documents.iter().fold(0, |a, &b| a + b.1) as f32 / documents.len() as f32;

        let docs_file = File::create("documents.bin")?;
        let mut docs_out = BufWriter::new(docs_file);

        write_documents(
            documents.len() as u32,
            avg_dl,
            documents.iter().map(|(name, term_count)| Document {
                term_count: *term_count,
                name: name.to_string(),
            }),
            &mut docs_out,
        )?;
    }

    // Write postings and blocks files concurrently
    let index = index.iter().collect::<Vec<_>>();

    {
        let post_file = File::create("postings.bin")?;
        let mut post_out = BufWriter::new(post_file);

        let block_file = File::create("blocks.bin")?;
        let mut block_out = BufWriter::new(block_file);

        let index_file = File::create("index.bin")?;
        let mut index_out = BufWriter::new(index_file);

        let mut postings_offset: usize = 0;
        let mut blocks_offset: usize = write_varint(&mut block_out, index.len() as u64)?;
        let mut index_offset: usize = write_varint(&mut index_out, (index.len() as u64) / 1000)?;
        let mut n = 0;

        for (term, postings) in index {
            let post_ptr = postings_offset;

            postings_offset += write_postings(
                postings.len() as u32,
                postings.iter().map(|(&document, &frequency)| Posting {
                    document,
                    frequency,
                }),
                &mut post_out,
            )?;

            let block_ptr = blocks_offset;

            blocks_offset += write_term(term.as_bytes(), post_ptr as u32, &mut block_out)?;

            // Write every 1000 terms to the root index
            if n % 1000 == 0 {
                index_offset += write_term(term.as_bytes(), block_ptr as u32, &mut index_out)?;
            }

            n += 1;
        }

        post_out.flush()?;
        block_out.flush()?;
        index_out.flush()?;
    }

    Ok(())
}
