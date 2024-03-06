use crate::{DataHeader, DataTypes, Id, IdCache};
use anyhow::Result;
use binrw::{BinReaderExt, NullString};
use std::{fs::File, io::{BufReader, Cursor, Read, Seek, SeekFrom}};

const BANK_KEY: [u8; 8] = [0xac, 0xbc, 0x11, 0x92, 0x38, 0x70, 0x10, 0xa3];

pub fn extract_bank(
    cache: &IdCache,
    bundle_id: &Id,
    d: &mut DataHeader,
    r: &mut BufReader<File>,
    sf: &mut Option<BufReader<File>>,
) -> Result<(Vec<u8>, Option<String>), anyhow::Error> {
    let mut path: Option<String> = None;
    let mut buf = if d.stream_data_size != 0 {
        d.get_stream_buf(sf)?
    } else {
        // todo: original bank hash is in here but cba to look right now
        let buf1 = d.get_bundle_buf(r)?;

        let mut c = Cursor::new(buf1.clone());
        c.seek(SeekFrom::Start(0x4))?;
        let bnk_size: u32 = c.read_le()?;
        let id: Id = c.read_be()?;

        let path_info = cache.get_by_id(id, Some(DataTypes::AudioPath), *bundle_id)?;

        // println!("{} {:?}", bundle_id, path_info);
        // println!("{:?}", path_info.1.data_offset + 8);
        r.seek(SeekFrom::Start(path_info.1.data_offset + 8))?;
        // let len: u32 = r.read_le()?;
        // let mut path_buf = vec![0u8;len as usize];
        // r.read_exact(&mut path_buf)?;
        let path_buf: NullString = r.read_ne()?;

        path = Some(path_buf.to_string());

        let mut buf2 = vec![0u8; bnk_size as usize];
        c.read_exact(&mut buf2)?;
        buf2
    };
    buf[8..0x10]
        .iter_mut()
        .zip(BANK_KEY.iter())
        .for_each(|(x1, x2)| *x1 ^= *x2);
    Ok((buf, path))
}

pub fn extract_wem(
    d: &mut DataHeader,
    r: &mut BufReader<File>,
    sf: &mut Option<BufReader<File>>,
) -> Result<(Vec<u8>, Option<String>), anyhow::Error> {
    let buf = d.get_stream_buf(sf)?;
    // note: this is not reversable unless theres a list of names somewhere in the files (https://github.com/bnnm/wwiser/blob/master/doc/WWISER.md#wwise-files-and-names)
    let hash_buf = d.get_bundle_buf(r)?.as_slice()[0x8..0xC].to_vec();
    let hash = u32::from_be_bytes(hash_buf.as_slice().try_into().unwrap());
    Ok((buf, Some(format!("{}", hash))))
}
