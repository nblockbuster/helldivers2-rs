use std::{collections::HashMap, io::{Seek, SeekFrom}};

use binrw::{BinRead, BinReaderExt, NullString};

use crate::{DataHeader, DataReaders};


#[derive(BinRead, Debug, Default, Clone)]
pub struct StringFile {
    pub unk0: u32,
    pub unk4: u32,
    pub string_count: u32,
    // TODO: check language to switch to UTF-16? (2427891497 - ) (also in file name itself)
    pub language_id: u32,

    #[br(count=string_count)]
    pub string_ids: Vec<u32>,

    #[br(count=string_count)]
    pub string_offsets: Vec<u32>,

    // here -> EOF: null-terminated strings
}

pub fn extract_strings(
    d: &mut DataHeader,
    r: &mut DataReaders,
) -> Result<(Vec<u8>, Option<String>), anyhow::Error> {
    let bundle = &mut r.bundle();
    bundle.seek(SeekFrom::Start(d.data_offset))?;

    let h: StringFile = bundle.read_le()?;

    let mut strings: HashMap<u32, String> = Default::default();

    for i in 0..h.string_count {
        let id = *h.string_ids.get(i as usize).unwrap();
        let offset = *h.string_offsets.get(i as usize).unwrap();

        bundle.seek(SeekFrom::Start(d.data_offset + offset as u64))?;
        let string: NullString = bundle.read_le()?;
        // println!("{}: {}", id, string);
        strings.insert(id, string.to_string());
    }

    Ok((serde_json::to_string_pretty(&strings)?.as_bytes().to_vec(), None))
}
