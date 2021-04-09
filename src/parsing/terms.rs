use std::borrow::Cow;

#[derive(Debug)]
enum Error {
    UnexpectedEOF,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Terms<'a> {
    buffer: &'a [u8],
    ptr: usize,
}

impl<'a> Terms<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Terms { buffer, ptr: 0 }
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
}

impl<'a> Iterator for Terms<'a> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        self.skip_whitespace();

        if self.peek().ok()?.is_ascii_alphanumeric() {
            let mut result = String::with_capacity(12);

            result.push(self.peek().ok()? as char);
            self.ptr += 1;

            // Basic handling of contractions
            loop {
                let next = match self.peek() {
                    Ok(c) => c as char,
                    Err(_) => break,
                };

                if next.is_ascii_alphanumeric() || next == '\'' {
                    result.push(next);
                    self.ptr += 1;
                } else {
                    break;
                }
            }

            Some(result.to_ascii_lowercase())
        } else {
            let start = self.ptr;

            loop {
                let next = match self.peek() {
                    Ok(c) => c as char,
                    Err(_) => break,
                };

                if !next.is_ascii_alphanumeric() && !next.is_ascii_whitespace() {
                    self.ptr += 1;
                } else {
                    break;
                }
            }

            Some(String::from_utf8(Vec::from(&self.buffer[start..self.ptr])).unwrap())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_alphanumeric() {
        let mut t = Terms::new("$123".as_bytes());
        assert_eq!(t.next(), Some(String::from("$")));
        assert_eq!(t.next(), Some(String::from("123")));
        assert_eq!(t.next(), None);

        let mut t = Terms::new("123$$123".as_bytes());
        assert_eq!(t.next(), Some(String::from("123")));
        assert_eq!(t.next(), Some(String::from("$$")));
        assert_eq!(t.next(), Some(String::from("123")));
        assert_eq!(t.next(), None);
    }

    #[test]
    fn contractions() {
        let mut t = Terms::new("a'ight ain't amn't aren't can't could've couldn't couldn't've didn't doesn't don't hasn't".as_bytes());

        let terms = [
            "a'ight",
            "ain't",
            "amn't",
            "aren't",
            "can't",
            "could've",
            "couldn't",
            "couldn't've",
            "didn't",
            "doesn't",
            "don't",
            "hasn't",
        ];

        assert_eq!(
            t.collect::<Vec<_>>(),
            terms.iter().map(|s| String::from(*s)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn basic_words() {
        let mut t = Terms::new(
            "John Blair was acquired last year by Reliance Capital Group Inc., which has been divesting itself of John Blair's major assets."
                .as_bytes(),
        );

        let terms = [
            "john",
            "blair",
            "was",
            "acquired",
            "last",
            "year",
            "by",
            "reliance",
            "capital",
            "group",
            "inc",
            ".,",
            "which",
            "has",
            "been",
            "divesting",
            "itself",
            "of",
            "john",
            "blair's",
            "major",
            "assets",
            ".",
        ];

        assert_eq!(
            t.collect::<Vec<_>>(),
            terms.iter().map(|s| String::from(*s)).collect::<Vec<_>>()
        );
    }
}
