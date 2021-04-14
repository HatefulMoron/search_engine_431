use lazy_static::lazy_static;
use regex::{Matches, Regex};

#[derive(Debug)]
enum Error {
    UnexpectedEOF,
}

type Result<T> = std::result::Result<T, Error>;

pub struct Terms<'a> {
    matches: Matches<'static, 'a>,
}

impl<'a> Terms<'a> {
    pub fn new(buffer: &'a str) -> Self {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"\w+(?:'\w+)?|[^\w\s]").unwrap();
        }
        Terms {
            matches: RE.find_iter(buffer),
        }
    }
}

impl<'a> Iterator for Terms<'a> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        loop {
            let m = self.matches.next()?;
            let s = m.as_str();

            if s.chars().any(|c| c.is_alphanumeric()) {
                return Some(m.as_str().to_ascii_lowercase());
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_alphanumeric() {
        let mut t = Terms::new("$123");
        assert_eq!(t.next(), Some(String::from("123")));
        assert_eq!(t.next(), None);

        let mut t = Terms::new("123$$123");
        assert_eq!(t.next(), Some(String::from("123")));
        assert_eq!(t.next(), Some(String::from("123")));
        assert_eq!(t.next(), None);
    }

    #[test]
    fn contractions() {
        let mut t = Terms::new(
            "a'ight ain't amn't aren't can't could've couldn't didn't doesn't don't hasn't",
        );

        let terms = [
            "a'ight", "ain't", "amn't", "aren't", "can't", "could've", "couldn't", "didn't",
            "doesn't", "don't", "hasn't",
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
                ,
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
        ];

        assert_eq!(
            t.collect::<Vec<_>>(),
            terms.iter().map(|s| String::from(*s)).collect::<Vec<_>>()
        );
    }
}
