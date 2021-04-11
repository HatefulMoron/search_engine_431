use std::collections::{BTreeMap, HashSet};
use std::io;
use std::io::{stdout, BufRead, BufReader, BufWriter, Bytes, Read, Write};

mod indexing;
use indexing::index::{write_documents, write_postings, write_term};
use indexing::string::AsciiString;
use std::fs::File;

fn main() -> std::io::Result<()> {
    let stdin = io::stdin();
    let mut content = Vec::new();
    stdin.lock().read_to_end(&mut content).unwrap();

    // DOCNO
    let mut documents: Vec<AsciiString> = Vec::new();

    // Term -> Postings
    // Dictionary is set of terms/keys
    let mut index: BTreeMap<AsciiString, HashSet<usize>> = BTreeMap::new();
    let mut lines = content.split(|b| *b == b'\n');

    loop {
        let line = match lines.next() {
            Some(l) => l,
            None => break,
        };

        if documents.is_empty() {
            documents.push(AsciiString(line));
            continue;
        }

        if line.is_empty() {
            match lines.next() {
                Some(l) => documents.push(AsciiString(l)),
                None => break,
            }
            continue;
        }

        assert!(!line.is_empty());

        match index.get_mut(&AsciiString(line)) {
            Some(vec) => {
                vec.insert(documents.len() - 1);
            }
            None => {
                let mut set = HashSet::with_capacity(32);
                set.insert(documents.len() - 1);
                index.insert(AsciiString(line), set);
            }
        };
    }

    // Write documents
    {
        let docs_file = File::create("documents.bin")?;
        let mut docs_out = BufWriter::new(docs_file);

        write_documents(
            documents.len() as u32,
            documents.iter().map(|s| s.as_bytes()),
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
        let mut blocks_offset: usize = 4;
        let mut index_offset: usize = 4;
        let mut n = 0;

        block_out.write_all(&(index.len() as u32).to_be_bytes()[..])?;
        index_out.write_all(&(index.len() as u32 / 1000).to_be_bytes()[..])?;

        for (term, postings) in index {
            println!("{}\t\t{}", term, postings.len());

            let post_ptr = postings_offset;

            postings_offset += write_postings(
                postings.len() as u32,
                postings.iter().map(|o| *o as u32),
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

        post_out.flush();
        block_out.flush();
        index_out.flush();
    }

    Ok(())
}
