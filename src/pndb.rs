use std::{collections::HashMap, fs::File, io::{BufReader, Cursor}};

use binrw::{BinRead, BinReaderExt, NullString};

use crate::Id;

#[derive(Debug, Default, Clone)]
pub struct Pndb {
    // pub name_database_length: u32,
    // pub compressed_length: u32,
    // pub decompressed_length: u32,
    // pub compressed_data: Vec<u8>,

    pub name_database: HashMap<Id, String>,
}

impl BinRead for Pndb {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let mut name_database = HashMap::new();
        let pos = reader.stream_position()?;
        let val = u32::read_options(reader, endian, ())?;
        if val != u32::from_le_bytes(*b"PNDB") {
            return Err(binrw::Error::BadMagic { pos, found: Box::new(val) as _ })
        }

        let database_len = u32::read_options(reader, endian, ())?;
        let compressed_len = u32::read_options(reader, endian, ())?;
        let decompressed_len = u32::read_options(reader, endian, ())?;

        let compressed_data: Vec<u8> = binrw::helpers::count_with(compressed_len as usize, u8::read_options)(reader, endian, args)?;

        let decompressed_data = lz4_flex::decompress(&compressed_data, decompressed_len as usize);
        if let Err(e) = decompressed_data {
            return Err(binrw::Error::AssertFail { pos, message: format!("Failed to decompress data: {}", e) })
        }

        let mut decompressed_reader = Cursor::new(decompressed_data.unwrap());

        let mut vals: Vec<String> = Vec::new();
        for _ in 0..database_len {
            let val: NullString = binrw::BinRead::read_options(&mut decompressed_reader, endian, ())?;
            vals.push(val.to_string());
        }

        for i in 0..database_len {
            let key: Id = Id::read_options(&mut decompressed_reader, endian, ())?;
            name_database.insert(key, vals.get(i as usize).unwrap().to_string());
        }

        Ok(Pndb { name_database })
    }
}

pub fn read_pndb(path: &str) -> anyhow::Result<Pndb> {
    let mut reader = BufReader::new(File::open(path)?);
    let pndb: Pndb = reader.read_le()?;
    Ok(pndb)
}
