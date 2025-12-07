use crate::{SectionData, addr::Addr};

#[derive(Debug, Clone)]
pub enum DataStringType {
    Utf8String(String),
    Utf16String(String),
}

#[derive(Debug, Clone)]
pub struct DataString {
    pub addr: Addr,
    pub typ: DataStringType,
}

impl DataStringType {
    pub fn len(&self) -> usize {
        match self {
            DataStringType::Utf8String(inner) => inner.len() + 1,
            DataStringType::Utf16String(inner) => inner.len() * 2 + 2,
        }
    }
}

pub fn collect_data_strings(section: SectionData<'_>) -> Vec<DataString> {
    let mut res = vec![];

    let mut addr = section.address;
    let mut data = section.data;

    while !data.is_empty() {
        if addr == Addr(0x715c18) {
            println!()
        }

        if let Some((s, size)) = read_utf8_or_win_1252(data) {
            res.push(DataString {
                addr,
                typ: DataStringType::Utf8String(s),
            });
            // It seems like string have padding after them
            let size = size.next_multiple_of(2).min(data.len());
            addr += size as u32;
            data = &data[size..];
            continue;
        }

        // 2. UTF-16 string?
        if let Some((s, size)) = read_utf16_string(data) {
            res.push(DataString {
                addr,
                typ: DataStringType::Utf16String(s),
            });

            let size = size.next_multiple_of(2).min(data.len());
            addr += size as u32;
            data = &data[size..];
            continue;
        }

        data = &data[2..];
        addr += 2;
    }

    res
}

pub fn read_utf8_or_win_1252(data: &[u8]) -> Option<(String, usize)> {
    let raw = data.split(|c| *c == 0).next()?;
    if raw.len() < 4 {
        return None;
    }
    // Try UTF-8 first
    if let Ok(s) = std::str::from_utf8(raw) {
        return Some((s.to_string(), raw.len() + 1));
    }

    None
    // TODO: this creates a lot of false positives
    // let (s, _, was_malformed) = encoding_rs::WINDOWS_1252.decode(raw);
    // if was_malformed {
    //     None
    // } else {
    //     Some((s.to_string(), raw.len() + 1))
    // }
}

pub fn read_utf16_string(data: &[u8]) -> Option<(String, usize)> {
    if data.len() < 2 {
        return None;
    }
    if data[1] != 0 {
        return None;
    } // must start with LE ASCII-range wide char

    let mut end = 0;
    while end + 1 < data.len() {
        if data[end] == 0 && data[end + 1] == 0 {
            let units: Vec<u16> = data[..end]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let s = String::from_utf16(&units).unwrap();
            if s.len() >= 4 {
                return Some((s, end + 2));
            } else {
                return None;
            }
        }
        if data[end + 1] != 0 {
            return None;
        } // only ASCII-range UTF-16 allowed here
        end += 2;
    }
    None
}
