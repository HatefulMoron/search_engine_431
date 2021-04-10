use super::string::AsciiString;
use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{Read, Write};

pub struct DiskIndex {
    source: File,
    root: BTreeMap<String, Block>,
}

enum Block {
    Loaded {
        map: BTreeMap<String, HashSet<Posting>>,
    },
    Unloaded {
        offset: usize,
    },
}

pub struct Posting {
    pub document_id: u32,
}

pub struct IndexElement {
    pub term: String,
    pub offset: u32,
}

pub fn write_postings<I: Iterator<Item = u32>, W: Write>(
    n: u32,
    mut iter: I,
    writer: &mut W,
) -> std::io::Result<usize> {
    writer.write_all(&n.to_be_bytes()[..])?;

    let mut offset: usize = 4;

    while let Some(posting) = iter.next() {
        let bin: [u8; 4] = posting.to_be_bytes();
        writer.write_all(&bin[..])?;

        offset += 4;
    }

    Ok(offset)
}

pub fn write_term<W: Write>(buf: &[u8], ptr: u32, writer: &mut W) -> std::io::Result<usize> {
    // Write string length
    writer.write_all(&(buf.len() as u32).to_be_bytes()[..])?;

    // Write string
    writer.write_all(buf)?;

    // Write ptr
    writer.write_all(&ptr.to_be_bytes()[..]);

    Ok(4 + buf.len() + 4)
}

pub fn read_postings<R: Read, C: Extend<u32>>(
    reader: &mut R,
    container: &mut C,
) -> std::io::Result<usize> {
    let len = {
        let mut len_bytes: [u8; 4] = [0; 4];
        reader.read_exact(&mut len_bytes[..])?;
        u32::from_be_bytes(len_bytes)
    };

    let mut offset: usize = 4;
    let mut postings = Vec::new();

    for _ in 0..len {
        let mut doc_id_bytes: [u8; 4] = [0; 4];
        reader.read_exact(&mut doc_id_bytes[..])?;

        let doc_id = u32::from_be_bytes(doc_id_bytes);
        postings.push(doc_id);
        offset += 4;
    }

    container.extend(postings);

    Ok(offset)
}
