use std::io;
use std::io::{Bytes, Read};

mod parsing;
use parsing::terms::Terms;
use parsing::tokens::{Token, Tokens};

fn main() {
    let stdin = io::stdin();
    let mut content = Vec::new();
    stdin.lock().read_to_end(&mut content).unwrap();

    let mut t = Tokens::new(content.as_slice());

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
                        println!("\n{}", String::from_utf8(id.to_vec()).unwrap());
                    }
                }
            }
            // https://en.wikipedia.org/wiki/List_of_XML_and_HTML_character_entity_references
            Token::Entity(data) => {
                println!(
                    "{}",
                    match data {
                        b"quot" => "\"",
                        b"amp" => "&",
                        b"apos" => "'",
                        b"lt" => "<",
                        b"gt" => ">",
                        _ => "",
                    }
                )
            }
            Token::Text(data) => {
                let mut terms = Terms::new(data);

                while let Some(term) = terms.next() {
                    println!("{}", term);
                }
            }
        }
    }
}
