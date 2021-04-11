use super::super::parsing::terms::Terms;
use super::string::AsciiString;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom, Write};
use crate::indexing::varint::{write_varint, read_varint};

pub struct DiskIndex {
    post_file: File,
    blocks_file: File,

    // Loaded from disk immediately
    docs: Vec<Document>,
    root: Vec<(String, u32)>,

    // Loaded on an as-needed basis during search
    blocks: BTreeMap<u32, Block>,
}

impl DiskIndex {
    pub fn from_disk() -> std::io::Result<DiskIndex> {
        let mut post_file = File::open("postings.bin")?;
        let mut blocks_file = File::open("blocks.bin")?;
        let mut documents_file = File::open("documents.bin")?;
        let mut index_file = File::open("index.bin")?;

        let docs = {
            let mut bytes = Vec::with_capacity(8192);
            documents_file.read_to_end(&mut bytes);

            let mut reader = Cursor::new(bytes);
            let mut buffer = Vec::with_capacity(8192);

            read_documents(&mut reader, &mut buffer)?;
            buffer
        };

        let root = {
            let mut bytes = Vec::with_capacity(8192);
            index_file.read_to_end(&mut bytes);

            let mut reader = Cursor::new(bytes);
            let mut buffer = Vec::with_capacity(8192);

            read_terms(&mut reader, &mut buffer)?;
            buffer
        };

        // For each root element, create a leaf node that we haven't loaded
        // from disk yet
        let mut blocks = BTreeMap::new();
        for (_, ptr) in &root {
            blocks.insert(*ptr, Block::Unloaded);
        }

        Ok(DiskIndex {
            post_file,
            blocks_file,
            docs,
            root,
            blocks,
        })
    }

    fn ensure_block_loaded(&mut self, ptr: u32) -> std::io::Result<()> {
        if let Some(Block::Loaded { block: _ }) = self.blocks.get(&ptr) {
            return Ok(());
        }

        self.blocks_file.seek(SeekFrom::Start(ptr as u64))?;

        let mut reader = BufReader::new(&mut self.blocks_file);
        let mut rows = Vec::with_capacity(1000);

        for _ in 0..1000 {
            let term = match read_term(&mut reader) {
                Ok(t) => t,
                Err(_) => break,
            };
            rows.push(term);
        }

        self.blocks.insert(ptr, Block::Loaded { block: rows });
        Ok(())
    }

    pub fn postings(&mut self, term: &String) -> std::io::Result<Vec<Posting>> {
        let ind = match self.root.binary_search_by_key(&term, |(a, b)| a) {
            Ok(k) => self.root[k].1.clone(),
            Err(k) => self.root[k - 1].1.clone(),
        };

        self.ensure_block_loaded(ind)?;

        if let Block::Loaded { block } = &self.blocks[&ind] {
            let ptr = match block.binary_search_by_key(&term, |(a, b)| a) {
                Ok(k) => block[k].1.clone(),
                Err(k) => return Ok(Vec::new()),
            };

            self.post_file.seek(SeekFrom::Start(ptr as u64))?;

            let mut reader = BufReader::new(&mut self.post_file);
            let mut postings = Vec::with_capacity(1024);

            read_postings(&mut reader, &mut postings)?;

            Ok(postings)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn search(&mut self, query: &String) -> std::io::Result<Vec<(String, f32)>> {
        // Document id -> w_dq
        let mut weights: HashMap<u32, f32> = HashMap::new();

        let mut t = Terms::new(query.as_bytes());
        while let Some(term) = t.next() {
            let postings = self.postings(&term)?;

            let idf_t = f32::log10((self.docs.len() as f32) / (postings.len() as f32));

            for posting in &postings {
                let tf_td = posting.frequency as f32
                    / (self.docs[posting.document as usize].term_count as f32);

                match weights.get_mut(&posting.document) {
                    Some(w) => *w += tf_td * idf_t,
                    None => {
                        weights.insert(posting.document, tf_td * idf_t);
                    }
                }
            }
        }

        let mut ret = weights.into_iter().collect::<Vec<(u32, f32)>>();
        ret.sort_by(|&a, &b| b.1.partial_cmp(&a.1).unwrap());

        Ok(ret
            .into_iter()
            .map(|p| (self.docs[p.0 as usize].name.clone(), p.1))
            .collect())
    }
}

enum Block {
    Loaded { block: Vec<(String, u32)> },
    Unloaded,
}

pub fn write_documents<I: Iterator<Item = Document>, W: Write>(
    n: u32,
    mut iter: I,
    writer: &mut W,
) -> std::io::Result<usize> {
    writer.write_all(&n.to_be_bytes()[..])?;

    let mut offset: usize = 4;

    while let Some(doc) = iter.next() {
        // Write term count
        writer.write_all(&doc.term_count.to_be_bytes()[..])?;

        // Write document name
        writer.write_all(&(doc.name.len() as u32).to_be_bytes()[..])?;
        writer.write_all(&doc.name.as_bytes()[..])?;

        offset += (8 + doc.name.as_bytes().len());
    }

    Ok(offset)
}

pub fn read_documents<R: Read, C: Extend<Document>>(
    reader: &mut R,
    container: &mut C,
) -> std::io::Result<usize> {
    let len = {
        let mut len_bytes: [u8; 4] = [0; 4];
        reader.read_exact(&mut len_bytes[..])?;
        u32::from_be_bytes(len_bytes)
    };

    let mut offset: usize = 4;
    let mut documents = Vec::new();

    for _ in 0..len {
        let term_count = {
            let mut bytes: [u8; 4] = [0; 4];
            reader.read_exact(&mut bytes[..])?;
            u32::from_be_bytes(bytes)
        };

        let len = {
            let mut len_bytes: [u8; 4] = [0; 4];
            reader.read_exact(&mut len_bytes[..])?;
            u32::from_be_bytes(len_bytes)
        };

        let bytes = {
            let mut container = Vec::with_capacity(len as usize);
            container.resize(len as usize, 0);
            reader.read_exact(container.as_mut_slice());
            container
        };

        offset += 8 + bytes.len();
        documents.push(Document {
            term_count,
            name: String::from_utf8(bytes).unwrap(),
        });
    }

    container.extend(documents);

    Ok(offset)
}

pub struct Posting {
    pub document: u32,
    pub frequency: u32,
}

pub struct Document {
    pub term_count: u32,
    pub name: String,
}

pub fn write_postings<I: Iterator<Item = Posting>, W: Write>(
    n: u32,
    mut iter: I,
    mut writer: &mut W,
) -> std::io::Result<usize> {
    writer.write_all(&n.to_be_bytes()[..])?;

    let mut offset: usize = 4;
    let mut previous: u32 = 0;

    while let Some(posting) = iter.next() {
        // Let the document ID be the diff
        assert!(posting.document >= previous);
        let diff: u32 = posting.document - previous;
        previous = posting.document;

        offset += write_varint(&mut writer, diff as u64)?;

        let bin: [u8; 4] = posting.frequency.to_be_bytes();
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

pub fn read_term<R: Read>(reader: &mut R) -> std::io::Result<(String, u32)> {
    let len = {
        let mut len_bytes: [u8; 4] = [0; 4];
        reader.read_exact(&mut len_bytes[..])?;
        u32::from_be_bytes(len_bytes)
    };

    let mut data = Vec::with_capacity(len as usize);
    data.resize(len as usize, 0);

    reader.read_exact(data.as_mut_slice())?;

    let ptr = {
        let mut len_bytes: [u8; 4] = [0; 4];
        reader.read_exact(&mut len_bytes[..])?;
        u32::from_be_bytes(len_bytes)
    };

    Ok((String::from_utf8(data).unwrap(), ptr))
}

pub fn read_terms<R: Read, C: Extend<(String, u32)>>(
    mut reader: &mut R,
    container: &mut C,
) -> std::io::Result<()> {
    let len = {
        let mut len_bytes: [u8; 4] = [0; 4];
        reader.read_exact(&mut len_bytes[..])?;
        u32::from_be_bytes(len_bytes)
    };

    let mut terms = Vec::new();
    for _ in 0..len {
        terms.push(read_term(&mut reader)?);
    }
    container.extend(terms);
    Ok(())
}

pub fn read_postings<R: Read, C: Extend<Posting>>(
    mut reader: &mut R,
    container: &mut C,
) -> std::io::Result<usize> {
    let len = {
        let mut len_bytes: [u8; 4] = [0; 4];
        reader.read_exact(&mut len_bytes[..])?;
        u32::from_be_bytes(len_bytes)
    };

    let mut offset: usize = 4;
    let mut previous: u32 = 0;
    let mut postings = Vec::new();

    for _ in 0..len {
        let (diff, off) = read_varint(&mut reader)?;
        offset += off;

        let document = (diff as u32) + previous;
        previous = document;

        let mut frequency_bytes: [u8; 4] = [0; 4];
        reader.read_exact(&mut frequency_bytes[..])?;
        let frequency = u32::from_be_bytes(frequency_bytes);

        postings.push(Posting {
            document,
            frequency,
        });
        offset += 4;
    }

    container.extend(postings);

    Ok(offset)
}
