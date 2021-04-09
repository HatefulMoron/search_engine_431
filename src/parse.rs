use std::io;
use std::io::{stdout, BufWriter, Bytes, Read, Write};

mod parsing;

use parsing::terms::Terms;
use parsing::tokens::{Token, Tokens};

fn main() {
    let stdin = io::stdin();
    let mut content = Vec::new();
    stdin.lock().read_to_end(&mut content).unwrap();

    let mut t = Tokens::new(content.as_slice());

    let stdout = stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut first = true;

    while let Some(token) = t.next() {
        match token {
            Token::Tag(tag) => {
                if tag.open
                    && String::from_utf8(tag.name.to_vec())
                        .unwrap()
                        .to_ascii_lowercase()
                        == "docno"
                {
                    // Try print the document id
                    // Note the extra newline to separate documents
                    if let Some(Token::Text(id)) = t.next() {
                        if first {
                            first = false;
                            writeln!(out, "{}", String::from_utf8(id.to_vec()).unwrap());
                        } else {
                            writeln!(out, "\n{}", String::from_utf8(id.to_vec()).unwrap());
                        }
                    }
                }
            }
            // https://en.wikipedia.org/wiki/List_of_XML_and_HTML_character_entity_references
            Token::Entity(data) => {
                writeln!(
                    out,
                    "{}",
                    match data {
                        b"quot" => "\"",
                        b"amp" => "&",
                        b"apos" => "'",
                        b"lt" => "<",
                        b"gt" => ">",
                        _ => "",
                    }
                );
            }
            Token::Text(data) => {
                let mut terms = Terms::new(data);

                while let Some(term) = terms.next() {
                    writeln!(out, "{}", term);
                }
            }
        }
    }
}
