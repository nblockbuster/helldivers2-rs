use crate::DataHeader;
use anyhow::Result;
use std::{fs::File, io::BufReader};

const BANK_KEY: [u8; 8] = [0xac, 0xbc, 0x11, 0x92, 0x38, 0x70, 0x10, 0xa3];

pub fn extract_bank(
    d: &mut DataHeader,
    r: &mut BufReader<File>,
    sf: &mut Option<BufReader<File>>,
) -> Result<(Vec<u8>, Option<String>), anyhow::Error> {
    let mut buf = if d.stream_data_size != 0 {
        d.get_stream_buf(sf)?
    } else {
        let buf1 = d.get_bundle_buf(r)?;
        let mut buf2 = vec![0u8; d.data_size as usize - 0x10];
        buf2.copy_from_slice(&buf1[0x10..d.data_size as usize]);
        buf2
    };
    buf[8..0x10]
        .iter_mut()
        .zip(BANK_KEY.iter())
        .for_each(|(x1, x2)| *x1 ^= *x2);
    Ok((buf, None))
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
