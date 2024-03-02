use std::{fs::File, io::BufReader};
use anyhow::Result;
use crate::DataHeader;

const BANK_KEY: [u8;8] = [0xac, 0xbc, 0x11, 0x92, 0x38, 0x70, 0x10, 0xa3];

pub fn extract_bank(
    d: &mut DataHeader,
    r: &mut BufReader<File>,
    sf: &mut Option<BufReader<File>>,
) -> Result<Vec<u8>, anyhow::Error> {
    let mut buf = if d.stream_data_size != 0 { d.get_stream_buf(sf)? } else { d.get_bundle_buf(r)? };
    buf[8..0x10].iter_mut().zip(BANK_KEY.iter()).for_each(|(x1, x2) | *x1 ^= *x2);
    Ok(buf)
}
