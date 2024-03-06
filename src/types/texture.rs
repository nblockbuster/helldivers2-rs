use std::{fs::File, io::{BufReader, Read, Seek, SeekFrom}};

use crate::DataHeader;

pub fn extract_texture(
    d: &mut DataHeader,
    r: &mut BufReader<File>,
    sf: &mut Option<BufReader<File>>,
    gf: &mut Option<BufReader<File>>,
) -> Result<(Vec<u8>, Option<String>), anyhow::Error> {
    let mut out_buf: Vec<u8> = Vec::new();
    // TODO: figure out what 0 -> c0 is for
    r.seek(SeekFrom::Start(d.data_offset + 0xc0))?;
    let mut dds_header = vec![0u8; 0x94];
    r.read_exact(&mut dds_header)?;
    out_buf.extend_from_slice(&dds_header);
    if d.stream_data_size > 0 {
        if sf.is_none() {
            return Err(anyhow::anyhow!("Stream file referenced but not found."));
        }
        if let Some(ref mut sf) = sf {
            sf.seek(SeekFrom::Start(d.stream_data_offset as u64))?;
            let mut data = vec![0u8; d.stream_data_size.try_into().unwrap()];
            sf.read_exact(&mut data)?;
            out_buf.extend_from_slice(&data);
        }
    } else {
        if gf.is_none() {
            return Err(anyhow::anyhow!(
                "GPU Resource file referenced but not found."
            ));
        }
        if let Some(ref mut gf) = gf {
            gf.seek(SeekFrom::Start(d.gpu_data_offset))?;
            let mut data = vec![0u8; d.gpu_data_size.try_into().unwrap()];
            gf.read_exact(&mut data)?;
            out_buf.extend_from_slice(&data);
        }
    }
    Ok((out_buf, None))
}