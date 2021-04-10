use std::cmp::Ordering;

#[derive(Debug)]
pub struct AsciiString<'a>(pub &'a [u8]);

impl<'a> AsciiString<'a> {
    pub fn as_bytes(&self) -> &'a [u8] {
        return self.0;
    }
}

impl<'a> std::fmt::Display for AsciiString<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in self.0 {
            write!(f, "{}", *b as char)?;
        }
        Ok(())
    }
}

impl<'a> Ord for AsciiString<'a> {
    fn cmp(&self, other: &AsciiString<'_>) -> Ordering {
        for pair in self.0.iter().zip(other.0.iter()) {
            // a b c | d e f
            // ^       ^
            let order = pair.0.cmp(pair.1);
            if order != Ordering::Equal {
                return order;
            }
        }

        // consider, "run" vs "running"
        if self.0.len() != other.0.len() {
            self.0.len().cmp(&other.0.len())
        } else {
            Ordering::Equal
        }
    }
}

impl<'a> PartialOrd for AsciiString<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> PartialEq for AsciiString<'a> {
    fn eq(&self, other: &Self) -> bool {
        if self.0.len() != other.0.len() {
            return false;
        }

        for pair in self.0.iter().zip(other.0.iter()) {
            if pair.0 != pair.1 {
                return false;
            }
        }

        true
    }
}

impl<'a> Eq for AsciiString<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascci_compare() {
        let hello = "hello".as_bytes();
        let world = "world".as_bytes();

        assert_eq!(AsciiString(&hello[..]) == AsciiString(&hello[..]), true);
        assert_eq!(AsciiString(&hello[..]) == AsciiString(&world[..]), false);
        assert_eq!(AsciiString(&world[..]) == AsciiString(&hello[..]), false);
        assert_eq!(AsciiString(&world[..]) == AsciiString(&world[..]), true);

        let run = "run".as_bytes();
        let running = "running".as_bytes();

        assert_eq!(AsciiString(&run[..]) == AsciiString(&running[..]), false);
    }

    #[test]
    fn ascii_order() {
        let run = "run".as_bytes();
        let running = "running".as_bytes();

        assert_eq!(AsciiString(&run[..]) < AsciiString(&running[..]), true);
        assert_eq!(AsciiString(&run[..]) > AsciiString(&running[..]), false);
        assert_eq!(AsciiString(&run[..]) == AsciiString(&running[..]), false);

        let mut words = [
            "combination",
            "supermarket",
            "sample",
            "writing",
            "memory",
            "obligation",
            "consequence",
            "criticism",
            "boyfriend",
            "customer",
            "virus",
            "statement",
            "knowledge",
            "throat",
            "night",
            "mixture",
        ]
        .iter()
        .map(|s| AsciiString(s.as_bytes()))
        .collect::<Vec<_>>();
        words.sort();

        let sorted = [
            "boyfriend",
            "combination",
            "consequence",
            "criticism",
            "customer",
            "knowledge",
            "memory",
            "mixture",
            "night",
            "obligation",
            "sample",
            "statement",
            "supermarket",
            "throat",
            "virus",
            "writing",
        ]
        .iter()
        .map(|s| AsciiString(s.as_bytes()))
        .collect::<Vec<_>>();

        assert_eq!(words, sorted);
    }
}
