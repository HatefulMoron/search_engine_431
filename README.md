## Building
A recent version of Rust is required to build the programs. The rust compiler
can be installed locally per user using [rustup](https://rustup.rs/). 

Compiling the programs is done by,
```commandline
$ cargo build --release
```

after which the compiled binaries are located in `./target/release`.

## Programs
### Parse

The parser takes an XML file in `stdin` and writes tokens to `stdout` in the
following format,

### Example

```commandline
$ ./target/release/parse < wsj.small.xml
WSJ870324-0001
john
blair
is
near
accord
to
sell
unit
,
```

### Index

#### Example,

```commandline
$ ./target/release/parse < wsj.xml | ./target/release/index
```

The indexer takes a sequence of tokens formatted similarly to the above snippet
and builds an index on disk. At the time of writing, the following files are
created by the index program,

| File | Purpose |
|------|---------|
| documents.bin | Contains snippets of information about each document.
| postings.bin | Stores each set of postings for each term in the index.
| blocks.bin | Leaves of ISAM B-Tree, each element pointing to an element in postings.bin. |
| index.bin | Root index of ISAM B-Tree, each element pointing to an element in blocks.bin.

All file formats are binary, and they are explained below,

#### documents.bin header format

| Type | Name | Notes |
|------|------|---------|
| varint | count | Number of documents indexed. |

#### documents.bin element format

| Type | Name | Notes |
|------|------|---------|
| varint | length | Size of document name in bytes. |
| UTF-8 bytes | name | Length given by previous value. |

#### postings.bin element format

| Type | Name | Notes |
|------|------|---------|
| varint | length | Number of postings associated with this term. |
| **repeated** | | |
| varint | diff | The difference between this posting's document ID and the previous posting's document ID. |
| varint | frequency | Raw frequency of the term inside the document given by this posting. |

#### blocks.bin element format

| Type | Name | Notes |
|------|------|---------|
| varint | length | Length of term in bytes.|
| UTF-8 bytes | term | |
| varint | ptr | File offset pointing to the matching posting in postings.bin. |

#### index.bin element format

| Type | Name | Notes |
|------|------|---------|
| varint | length | Length of term in bytes.|
| UTF-8 bytes | term | |
| varint | ptr | File offset pointing to the matching posting in blocks.bin. |

### Search

The search program expects to have the aforementioned binary files inside the
directory it is being executed in.

#### Example,

```commandline
$ echo "thomas" | ./target/release/search > out.txt
$ head out.txt
WSJ880222-0099 0.06406918
WSJ871224-0043 0.050482526
WSJ880921-0145 0.047611043
WSJ870302-0127 0.04670643
WSJ881103-0190 0.04633575
WSJ870106-0023 0.044737965
WSJ871117-0113 0.04422958
WSJ900406-0010 0.0432467
WSJ870720-0147 0.039716356
WSJ870603-0045 0.039615296
```
