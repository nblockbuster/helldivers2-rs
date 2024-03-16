use crate::{DataHeader, DataReaders, DataTypes, Id, IdCache};
use anyhow::Result;
use binrw::{BinReaderExt, NullString};
use std::io::{Cursor, Read, Seek, SeekFrom};

const BANK_KEY: [u8; 8] = [0xac, 0xbc, 0x11, 0x92, 0x38, 0x70, 0x10, 0xa3];

pub fn extract_bank(
    cache: &IdCache,
    bundle_id: &Id,
    d: &mut DataHeader,
    r: &mut DataReaders,
) -> Result<(Vec<u8>, Option<String>), anyhow::Error> {
    let mut path: Option<String> = None;
    let mut buf = if d.stream_data_size != 0 {
        d.get_stream_buf(r.stream())?
    } else {
        let bundle = &mut r.bundle();
        let buf1 = d.get_bundle_buf(bundle)?;

        let mut c = Cursor::new(buf1.clone());
        c.seek(SeekFrom::Start(0x4))?;
        let bnk_size: u32 = c.read_le()?;
        let id: Id = c.read_be()?;

        let path_info = cache.get_by_id(id, Some(DataTypes::AudioPath), *bundle_id)?;
        // println!("{} {:?}", bundle_id, path_info);
        // println!("{:?}", path_info.1.data_offset + 8);
        bundle.seek(SeekFrom::Start(path_info.1.data_offset + 8))?;
        // let len: u32 = r.read_le()?;
        // let mut path_buf = vec![0u8;len as usize];
        // r.read_exact(&mut path_buf)?;
        let path_buf: NullString = bundle.read_ne()?;

        path = if path_buf.is_empty() {
            Some(d.unk_id.to_string())
        } else {
            Some(path_buf.to_string())
        };

        // path = Some(path_buf.to_string());
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
    r: &mut DataReaders,
) -> Result<(Vec<u8>, Option<String>), anyhow::Error> {
    let buf = d.get_stream_buf(r.stream())?;
    Ok((buf, None))
}
