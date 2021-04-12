use super::super::parsing::terms::Terms;

use crate::indexing::varint::{read_varint, write_varint};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom, Write};

use ordered_float::OrderedFloat;
use smallvec::SmallVec;

struct DiskDocument {
    term_count: u32,
    name: SmallVec<[u8; 32]>
}

pub struct DiskIndex {
    post_file: File,
    blocks_file: File,

    // Loaded from disk immediately
    docs: Vec<DiskDocument>,
    root: Vec<(String, u32)>,

    // Loaded on an as-needed basis during search
    blocks: BTreeMap<u32, Block>,
}

impl DiskIndex {
    pub fn from_disk() -> std::io::Result<DiskIndex> {
        let post_file = File::open("postings.bin")?;
        let blocks_file = File::open("blocks.bin")?;
        let mut documents_file = File::open("documents.bin")?;
        let mut index_file = File::open("index.bin")?;

        let docs = {
            let mut bytes = Vec::with_capacity(8192);
            documents_file.read_to_end(&mut bytes)?;

            let mut reader = Cursor::new(bytes);
            let mut buffer = Vec::with_capacity(8192);

            read_documents(&mut reader, &mut buffer)?;
            buffer
        };

        let root = {
            let mut bytes = Vec::with_capacity(8192);
            index_file.read_to_end(&mut bytes)?;

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

    // Ensure that the block given by the file offset `ptr` is loaded in
    // memory.
    fn ensure_block_loaded(&mut self, ptr: u32) -> std::io::Result<()> {
        // If the block is already loaded, no IO needs to be done.
        if let Some(Block::Loaded { block: _ }) = self.blocks.get(&ptr) {
            return Ok(());
        }

        // Seek to the offset given by `ptr` from the beginning of the file.
        self.blocks_file.seek(SeekFrom::Start(ptr as u64))?;

        let mut reader = BufReader::new(&mut self.blocks_file);
        let mut rows = Vec::with_capacity(1000);

        // Every block except the last block is exactly 1000 elements, so size
        // information is not necessary. The last block will result in an EOF
        // IO error which we can use to break early.
        for _ in 0..1000 {
            let term = match read_term(&mut reader) {
                Ok(t) => t,
                Err(_) => break,
            };
            rows.push(term);
        }

        // Insert the block for any subsequent calls.
        self.blocks.insert(ptr, Block::Loaded { block: rows });

        Ok(())
    }

    // Returns the set of postings for a given `term`. This function results
    // in a disk read in the postings file.
    pub fn postings(&mut self, term: &String) -> std::io::Result<Vec<Posting>> {
        // Binary search the root index for `term`.
        // Note that because the root index is incomplete, it's likely that the
        // term isn't in the root index.
        // `binary_search_by_key` returns Err(k) when this happens, where
        // `k` is the index where this element could be inserted to avoid
        // disordering the structure.
        // Because the structure is sorted alphabetically[1], we know that
        // the previous element points to the block that would contain this
        // term.
        //
        // [1]: https://doc.rust-lang.org/std/cmp/trait.Ord.html
        // "When derived on structs, it will produce a lexicographic ordering
        //  based on the top-to-bottom declaration order of the struct's
        //  members."
        let ind = match self.root.binary_search_by_key(&term, |(a, _)| a) {
            Ok(k) => self.root[k].1.clone(),
            Err(k) => {
                if k > 0 {
                    self.root[k - 1].1.clone()
                } else {
                    self.root[0].1.clone()
                }
            }
        };

        // The given block needs to be loaded before we use it. We don't
        // necessarily need to read the block from disk -- for instance if the
        // block was previously loaded it will already be present.
        self.ensure_block_loaded(ind)?;

        if let Block::Loaded { block } = &self.blocks[&ind] {
            // Binary search within the block to find the term.
            // If the term isn't present, we definitely don't have any postings
            // for the term and can return early.
            let ptr = match block.binary_search_by_key(&term, |(a, _)| a) {
                Ok(k) => block[k].1.clone(),
                Err(_) => return Ok(Vec::new()),
            };

            // Seek in the postings file using `ptr` as the offset from the
            // beginning of the file.
            self.post_file.seek(SeekFrom::Start(ptr as u64))?;

            let mut reader = BufReader::new(&mut self.post_file);
            let mut postings = Vec::with_capacity(1024);

            read_postings(&mut reader, &mut postings)?;

            Ok(postings)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn document(&self, doc: u32) -> &str {
        std::str::from_utf8(self.docs[doc as usize].name.as_slice()).unwrap()
    }

    pub fn search(
        &mut self,
        query: &String,
    ) -> std::io::Result<impl Iterator<Item = (f32, u32)>> {
        // Document id -> w_dq
        let mut weights: HashMap<u32, f32> = HashMap::new();
        weights.reserve(self.docs.len());

        let mut t = Terms::new(query.as_bytes());

        // The standard tf-idf equation is written in terms of the weights per
        // document, which is backwards from how we store the postings[1],
        // so we keep a hashmap relating document IDs with the accumulated
        // weight and iterate through each term of the query.
        //
        // [1]: postings are stored in terms of the terms themselves, with a
        // subsequent list of documents.

        while let Some(term) = t.next() {
            let postings = self.postings(&term)?;

            // idf_t = log(N / N_{t})
            // where    N       = total number of documents,
            //          N_{t}   = number of documents with term `t` present
            let idf_t = f32::log10((self.docs.len() as f32) / ((postings.len() + 1) as f32));

            for posting in &postings {
                let tf_td = posting.frequency as f32
                    / (self.docs[posting.document as usize].term_count as f32);

                let w = weights.entry(posting.document).or_insert(0.0);
                *w += tf_td * idf_t;
            }
        }

        let res: BTreeMap<OrderedFloat<f32>, u32> = weights
            .into_iter()
            .map(|(doc, w)| {
                (OrderedFloat(w), doc)
            })
            .collect();

        Ok(res.into_iter().map(|(w, n)| (w.0, n)).rev())
    }
}

enum Block {
    Loaded { block: Vec<(String, u32)> },
    Unloaded,
}

// +-----------------+
// | N      (varint) |
// +-----------------+
// ..
// +---------------------+----------------------+-----------------------+
// | Term Count (varint) | Name Length (varint) | Document Name (bytes) |
// +---------------------+----------------------+-----------------------+
// ..
// (N times)
pub fn write_documents<I: Iterator<Item = Document>, W: Write>(
    n: u32,
    mut iter: I,
    mut writer: &mut W,
) -> std::io::Result<usize> {
    let mut offset = write_varint(&mut writer, n as u64)?;

    while let Some(doc) = iter.next() {
        // Write term count
        offset += write_varint(&mut writer, doc.term_count as u64)?;

        // Write document name
        offset += write_varint(&mut writer, doc.name.len() as u64)?;
        writer.write_all(&doc.name.as_bytes()[..])?;

        offset += doc.name.as_bytes().len();
    }

    Ok(offset)
}

pub fn read_documents<R: Read, C: Extend<DiskDocument>>(
    mut reader: &mut R,
    container: &mut C,
) -> std::io::Result<usize> {
    let (len, mut offset) = read_varint(&mut reader)?;

    let mut documents = Vec::new();

    for _ in 0..len {
        let (term_count, term_count_offset) = read_varint(&mut reader)?;
        let (len, len_offset) = read_varint(&mut reader)?;

        let bytes = {
            let mut container = SmallVec::<[u8; 32]>::new();
            container.resize(len as usize, 0);
            reader.read_exact(container.as_mut_slice())?;
            container
        };

        offset += term_count_offset + len_offset + bytes.len();

        documents.push(DiskDocument {
            term_count: term_count as u32,
            name: bytes,
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
    let mut offset = write_varint(&mut writer, n as u64)?;
    let mut previous: u32 = 0;

    while let Some(posting) = iter.next() {
        // Let the document ID be the diff
        assert!(posting.document >= previous);
        let diff: u32 = posting.document - previous;
        previous = posting.document;

        offset += write_varint(&mut writer, diff as u64)?;
        offset += write_varint(&mut writer, posting.frequency as u64)?;
    }

    Ok(offset)
}

pub fn write_term<W: Write>(buf: &[u8], ptr: u32, mut writer: &mut W) -> std::io::Result<usize> {
    // Write string length
    let mut offset = write_varint(&mut writer, buf.len() as u64)?;

    // Write string
    writer.write_all(buf)?;
    offset += buf.len();

    // Write ptr
    offset += write_varint(&mut writer, ptr as u64)?;

    Ok(offset)
}

pub fn read_term<R: Read>(mut reader: &mut R) -> std::io::Result<(String, u32)> {
    let (len, _offset) = read_varint(&mut reader)?;

    let mut data = Vec::with_capacity(len as usize);
    data.resize(len as usize, 0);

    reader.read_exact(data.as_mut_slice())?;

    let (ptr, _offset) = read_varint(&mut reader)?;

    Ok((String::from_utf8(data).unwrap(), ptr as u32))
}

pub fn read_terms<R: Read, C: Extend<(String, u32)>>(
    mut reader: &mut R,
    container: &mut C,
) -> std::io::Result<()> {
    let (len, _offset) = read_varint(&mut reader)?;

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
    let (len, mut offset) = read_varint(&mut reader)?;

    let mut previous: u32 = 0;
    let mut postings = Vec::new();

    for _ in 0..len {
        let (diff, off) = read_varint(&mut reader)?;
        offset += off;

        let document = (diff as u32) + previous;
        previous = document;

        let (frequency, off) = read_varint(&mut reader)?;
        offset += off;

        postings.push(Posting {
            document,
            frequency: frequency as u32,
        });
        offset += 4;
    }

    container.extend(postings);

    Ok(offset)
}
