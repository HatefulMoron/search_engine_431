use std::io::{Read, Write};

// The writing procedure is from the MIDI format specification
pub fn write_varint<W: Write>(writer: &mut W, mut v: u64) -> std::io::Result<usize> {
    if v == 0 {
        writer.write(&[0x00])?;
        Ok(1)
    } else {
        let mut offset = 0;
        let mut buffer: u64 = v & 0x7f;

        while (v >> 7) > 0 {
            v >>= 7;
            buffer <<= 8;
            buffer |= 0x80;
            buffer += v & 0x7f;
        }

        loop {
            writer.write_all(&[buffer as u8])?;
            offset += 1;

            if buffer & 0x80 > 0 {
                buffer >>= 8;
            } else {
                break;
            }
        }

        Ok(offset)
    }
}

pub fn read_varint<R: Read>(reader: &mut R) -> std::io::Result<(u64, usize)> {
    let mut offset: usize = 0;
    let mut result: u64 = 0;
    let mut octet: [u8; 1] = [0; 1];

    loop {
        reader.read_exact(&mut octet[..])?;
        offset += 1;

        result = (result << 7) | (octet[0] & 0b0111_1111) as u64;

        if octet[0] & 0b1000_0000 == 0 {
            return Ok((result, offset));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read() {
        let examples = [
            (vec![0x00], 0),
            (vec![0x7F], 127),
            (vec![0x81, 0x00], 128),
            (vec![0xC0, 0x00], 8192),
            (vec![0xFF, 0x7F], 16383),
            (vec![0x81, 0x80, 0x00], 16384),
            (vec![0xFF, 0xFF, 0x7F], 2097151),
            (vec![0x81, 0x80, 0x80, 0x00], 2097152),
            (vec![0xC0, 0x80, 0x80, 0x00], 134217728),
            (vec![0xFF, 0xFF, 0xFF, 0x7F], 268435455),
        ];

        for (data, result) in examples.iter() {
            let mut reader = Cursor::new(data);

            assert_eq!(read_varint(&mut reader).unwrap(), (*result, data.len()));
        }
    }

    #[test]
    fn write() {
        let examples: [(Vec<u8>, u64); 10] = [
            (vec![0x00], 0),
            (vec![0x7F], 127),
            (vec![0x81, 0x00], 128),
            (vec![0xC0, 0x00], 8192),
            (vec![0xFF, 0x7F], 16383),
            (vec![0x81, 0x80, 0x00], 16384),
            (vec![0xFF, 0xFF, 0x7F], 2097151),
            (vec![0x81, 0x80, 0x80, 0x00], 2097152),
            (vec![0xC0, 0x80, 0x80, 0x00], 134217728),
            (vec![0xFF, 0xFF, 0xFF, 0x7F], 268435455),
        ];

        for (data, result) in examples.iter() {
            println!("result: {}", result);

            let mut buf = Vec::new();
            let mut writer = Cursor::new(buf);
            write_varint(&mut writer, *result);
            assert_eq!(&writer.into_inner(), data);
        }
    }
}
