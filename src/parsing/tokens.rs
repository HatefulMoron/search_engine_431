use std::io;
use std::io::{Bytes, Read};

#[derive(Debug)]
enum Error {
    Io(std::io::Error),
    ExpectedOpenBrace,
    ExpectedCloseBrace,
    ExpectedAmpersand,
    UnexpectedClosingTag,
    UnexpectedEOF,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Tag<'a> {
    pub open: bool,
    pub name: &'a [u8],
}

#[derive(Debug)]
pub enum Token<'a> {
    Text(&'a [u8]),
    Tag(Tag<'a>),
    Entity(&'a [u8]),
}

impl<'a> std::fmt::Display for Token<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Text(data) => {
                write!(f, "text: '{}'", String::from_utf8(data.to_vec()).unwrap())
            }
            Token::Tag(tag) => {
                write!(
                    f,
                    "tag: {}, '{}'",
                    tag.open,
                    String::from_utf8(tag.name.to_vec()).unwrap()
                )
            }
            Token::Entity(name) => {
                write!(f, "entity '{}'", String::from_utf8(name.to_vec()).unwrap())
            }
        }
    }
}

#[derive(Debug)]
pub struct Tokens<'a> {
    buffer: &'a [u8],
    ptr: usize,
}

impl<'a> Tokens<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Tokens { buffer, ptr: 0 }
    }

    fn peek(&self) -> Result<u8> {
        if self.ptr >= self.buffer.len() {
            Err(Error::UnexpectedEOF)
        } else {
            Ok(self.buffer[self.ptr])
        }
    }

    fn skip_whitespace(&mut self) {
        while self.ptr < self.buffer.len() {
            if !self.buffer[self.ptr].is_ascii_whitespace() {
                break;
            } else {
                self.ptr += 1;
            }
        }
    }

    fn read_tag(&mut self) -> Result<Tag<'a>> {
        if self.peek()? != b'<' {
            return Err(Error::ExpectedOpenBrace);
        }

        self.ptr += 1;

        let (open, start) = if self.peek()? == b'/' {
            (false, self.ptr + 1)
        } else {
            (true, self.ptr)
        };

        while self.peek()? != b'>' {
            self.ptr += 1;
        }

        // Skip over the end
        self.ptr += 1;

        Ok(Tag {
            open,
            name: &self.buffer[start..self.ptr - 1],
        })
    }

    fn read_entity(&mut self) -> Result<&'a [u8]> {
        if self.peek()? != b'&' {
            return Err(Error::ExpectedAmpersand);
        }

        self.ptr += 1;
        let start = self.ptr;

        while self.peek()? != b';' {
            self.ptr += 1;
        }

        // Skip over ';'
        self.ptr += 1;
        Ok(&self.buffer[start..self.ptr - 1])
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        self.skip_whitespace();

        if self.peek().ok()? == b'<' {
            Some(Token::Tag(self.read_tag().ok()?))
        } else if self.peek().ok()? == b'&' {
            Some(Token::Entity(self.read_entity().ok()?))
        } else {
            // Otherwise, read until we find a '<' or '&'
            let start = self.ptr;
            let mut end = self.ptr;

            loop {
                let c = self.peek().ok()?;

                if c == b'<' || c == b'&' {
                    break;
                }

                if !c.is_ascii_whitespace() {
                    end = self.ptr;
                }

                self.ptr += 1;
            }

            Some(Token::Text(&self.buffer[start..=end]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_next_tag<'a>(t: &mut Tokens<'a>, open: bool, name: &'a str) {
        if let Token::Tag(tag) = t.next().unwrap() {
            assert_eq!(tag.open, open);
            assert_eq!(String::from_utf8(tag.name.to_vec()).unwrap(), name);
        } else {
            panic!("Not tag");
        }
    }

    fn assert_next_text<'a>(t: &mut Tokens<'a>, text: &'a str) {
        if let Token::Text(data) = t.next().unwrap() {
            assert_eq!(String::from_utf8(data.to_vec()).unwrap(), text);
        } else {
            panic!("Not tag");
        }
    }

    fn assert_next_entity<'a>(t: &mut Tokens<'a>, text: &'a str) {
        if let Token::Entity(data) = t.next().unwrap() {
            assert_eq!(String::from_utf8(data.to_vec()).unwrap(), text);
        } else {
            panic!("Not tag");
        }
    }

    #[test]
    fn read_tags() {
        let mut t = Tokens::new("  <tag> </TAG>".as_bytes());

        assert_next_tag(&mut t, true, "tag");
        assert_next_tag(&mut t, false, "TAG");

        assert!(t.next().is_none());
    }

    #[test]
    fn unfinished_tag() {
        let mut t = Tokens::new("  <forgottoend".as_bytes());
        assert!(t.next().is_none());
    }

    #[test]
    fn read_entity() {
        let mut t = Tokens::new(" &amp;".as_bytes());

        assert_next_entity(&mut t, "amp");

        assert!(t.next().is_none());
    }

    #[test]
    fn tag_content() {
        let mut t = Tokens::new("<name>thomas</name>".as_bytes());

        assert_next_tag(&mut t, true, "name");
        assert_next_text(&mut t, "thomas");
        assert_next_tag(&mut t, false, "name");

        assert!(t.next().is_none());
    }

    #[test]
    fn text_useless_whitespace() {
        let mut t = Tokens::new("<DOCNO> WSJ870324-0001 </DOCNO>".as_bytes());

        assert_next_tag(&mut t, true, "DOCNO");
        assert_next_text(&mut t, "WSJ870324-0001");
        assert_next_tag(&mut t, false, "DOCNO");

        assert!(t.next().is_none());
    }

    #[test]
    fn example_document() {
        let mut t = Tokens::new(r#"
<DOC>
<DOCNO> WSJ870324-0001 </DOCNO>
<HL> John Blair Is Near Accord
To Sell Unit, Sources Say</HL>
<DD> 03/24/87</DD>
<SO> WALL STREET JOURNAL (J)</SO>
<IN> REL
TENDER OFFERS, MERGERS, ACQUISITIONS (TNM)
MARKETING, ADVERTISING (MKT)
TELECOMMUNICATIONS, BROADCASTING, TELEPHONE, TELEGRAPH (TEL) </IN>
<DATELINE> NEW YORK </DATELINE>
<TEXT>
   John Blair &amp; Co. is close to an agreement to sell its TV station advertising representation operation and program production unit to an investor group led by James H. Rosenfield, a former CBS Inc. executive, industry sources said.

   Industry sources put the value of the proposed acquisition at more than $100 million.
John Blair was acquired last year by Reliance Capital Group Inc., which has been divesting itself of John Blair's major assets.
John Blair represents about 130 local television stations in the placement of national and other advertising.

   Mr. Rosenfield stepped down as a senior executive vice president of CBS Broadcasting in December 1985 under a CBS early retirement program.
Neither Mr. Rosenfield nor officials of John Blair could be reached for comment.

</TEXT>
</DOC>
        "#.as_bytes());

        assert_next_tag(&mut t, true, "DOC");
        assert_next_tag(&mut t, true, "DOCNO");
        assert_next_text(&mut t, "WSJ870324-0001");
        assert_next_tag(&mut t, false, "DOCNO");
        assert_next_tag(&mut t, true, "HL");
        assert_next_text(
            &mut t,
            r#"John Blair Is Near Accord
To Sell Unit, Sources Say"#,
        );
        assert_next_tag(&mut t, false, "HL");
        assert_next_tag(&mut t, true, "DD");
        assert_next_text(&mut t, "03/24/87");
        assert_next_tag(&mut t, false, "DD");
        assert_next_tag(&mut t, true, "SO");
        assert_next_text(&mut t, "WALL STREET JOURNAL (J)");
        assert_next_tag(&mut t, false, "SO");
        assert_next_tag(&mut t, true, "IN");
        assert_next_text(
            &mut t,
            r#"REL
TENDER OFFERS, MERGERS, ACQUISITIONS (TNM)
MARKETING, ADVERTISING (MKT)
TELECOMMUNICATIONS, BROADCASTING, TELEPHONE, TELEGRAPH (TEL)"#,
        );
        assert_next_tag(&mut t, false, "IN");
        assert_next_tag(&mut t, true, "DATELINE");
        assert_next_text(&mut t, "NEW YORK");
        assert_next_tag(&mut t, false, "DATELINE");
        assert_next_tag(&mut t, true, "TEXT");
        assert_next_text(&mut t, "John Blair");
        assert_next_entity(&mut t, "amp");
        assert_next_text(
            &mut t,
            r#"Co. is close to an agreement to sell its TV station advertising representation operation and program production unit to an investor group led by James H. Rosenfield, a former CBS Inc. executive, industry sources said.

   Industry sources put the value of the proposed acquisition at more than $100 million.
John Blair was acquired last year by Reliance Capital Group Inc., which has been divesting itself of John Blair's major assets.
John Blair represents about 130 local television stations in the placement of national and other advertising.

   Mr. Rosenfield stepped down as a senior executive vice president of CBS Broadcasting in December 1985 under a CBS early retirement program.
Neither Mr. Rosenfield nor officials of John Blair could be reached for comment."#,
        );
        assert_next_tag(&mut t, false, "TEXT");
        assert_next_tag(&mut t, false, "DOC");
    }
}
