## Overview

### Precision

The trec_eval output using the 50 provided queries is given below,
```text
runid                 	all	thomas-passmore
num_q                 	all	50
num_ret               	all	933493
num_rel               	all	6228
num_rel_ret           	all	5408
map                   	all	0.2213
gm_map                	all	0.1111
Rprec                 	all	0.2483
bpref                 	all	0.3079
recip_rank            	all	0.5984
iprec_at_recall_0.00  	all	0.6442
iprec_at_recall_0.10  	all	0.4257
iprec_at_recall_0.20  	all	0.3441
iprec_at_recall_0.30  	all	0.2981
iprec_at_recall_0.40  	all	0.2448
iprec_at_recall_0.50  	all	0.2097
iprec_at_recall_0.60  	all	0.1759
iprec_at_recall_0.70  	all	0.1346
iprec_at_recall_0.80  	all	0.0954
iprec_at_recall_0.90  	all	0.0578
iprec_at_recall_1.00  	all	0.0122
P_5                   	all	0.4120
P_10                  	all	0.3960
P_15                  	all	0.3680
P_20                  	all	0.3550
P_30                  	all	0.3327
P_100                 	all	0.2512
P_200                 	all	0.1901
P_500                 	all	0.1177
P_1000                	all	0.0736
```

### Speed

The search program is able to perform all 50 queries on a single core in less
than a second on my machine,
```commandline
time ./target/release/search < wsj.51-100.titles.queries --trec > out.txt

real    0m0.296s
user    0m0.257
sys     0m0.034s
```

### Compression

In total, the index requires about 90MiB of storage.

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
..
```

.. where the first line is used as the document identifier, and the following
lines are interpreted by the indexing program as terms in the document.
Empty lines are used to indicate the end of each document.

### Index

#### Prebuilt Index
I've built a premade index located at,
```text
/home/cshome/t/tpassmore/search_engine_431
```

As noted below, the index files which the search engine uses are
`index.bin`, `blocks.bin`, `postings.bin` and `documents.bin`.

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
| 32bit float (big endian) | avg_dl | Average document length in terms. |

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
WSJ911011-0071 5.439788
WSJ911016-0124 5.439651
WSJ910905-0063 5.437519
WSJ911011-0017 5.424574
WSJ910702-0078 5.413733
WSJ910717-0008 5.4128447
WSJ911010-0117 5.4057636
WSJ911009-0154 5.3985877
WSJ910702-0086 5.3945026
WSJ911014-0005 5.3926396
```

The search program also supports an optional flag for consuming a TREC query
formatted file,

```commandline
$ ./target/release/search < wsj.51-100.titles.queries --trec > out.txt
$ head out.txt
51 Q0 WSJ871218-0126 0 18.04851 thomas-passmore
51 Q0 WSJ870204-0011 0 17.34641 thomas-passmore
51 Q0 WSJ900720-0157 0 17.28592 thomas-passmore
51 Q0 WSJ910708-0061 0 17.284029 thomas-passmore
51 Q0 WSJ871028-0094 0 17.07382 thomas-passmore
51 Q0 WSJ871012-0049 0 17.016558 thomas-passmore
51 Q0 WSJ920116-0130 0 16.986607 thomas-passmore
51 Q0 WSJ880321-0045 0 16.938957 thomas-passmore
51 Q0 WSJ861222-0013 0 16.90653 thomas-passmore
51 Q0 WSJ870727-0010 0 16.894127 thomas-passmore
```
