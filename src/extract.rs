// use crate::types::unit::*;

use super::structs::*;
use binrw::BinReaderExt;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

// TODO: combine extract_single and extract_files, using a vec of ids instead of singular id / bundle id?

pub fn extract_single(
    cache: &IdCache,
    output_path: &String,
    data_path: &String,
    id: Id,
    namedb: &crate::pndb::Pndb,
) -> anyhow::Result<()> {
    let (bundle_id, h) = cache.get_by_id(id, None, Id::invalid())?;
    let path = Path::new(data_path).join(bundle_id.to_string());
    if !path.exists() {
        return Err(anyhow::anyhow!("tried to open nonexistent file {:?}", path));
    }
    // println!("{:?}", path);
    let reader = BufReader::new(File::open(path)?);
    let mut out_path = PathBuf::from(output_path); //.join(bundle_file);

    let mut stream_path = Path::new(&data_path).join(bundle_id.to_string());
    stream_path.set_extension("stream");
    let mut stream_reader: Option<BufReader<File>> = None;
    let mut stream_size = 0;
    if stream_path.exists() {
        let file = File::open(stream_path.clone())?;
        stream_size = file.metadata()?.len();
        stream_reader = Some(BufReader::new(file));
    }

    let mut gpu_path = Path::new(&data_path).join(bundle_id.to_string());
    gpu_path.set_extension("gpu_resources");
    let mut gpu_reader: Option<BufReader<File>> = None;
    let mut gpu_size = 0;
    if gpu_path.exists() {
        let file = File::open(gpu_path.clone())?;
        gpu_size = file.metadata()?.len();
        gpu_reader = Some(BufReader::new(file));
    }
    let mut d: DataHeader = h.into();

    let mut readers: DataReaders = DataReaders(reader, stream_reader, gpu_reader);

    if export_special(
        cache,
        &Id::invalid(),
        &mut d,
        &mut readers,
        &out_path,
        namedb,
    )? {
        return Ok(());
    }

    let mut bundle_buf: Vec<u8> = Vec::new();
    let mut stream_buf: Vec<u8> = Vec::new();
    let mut gpu_buf: Vec<u8> = Vec::new();
    if h.data_size != 0 {
        bundle_buf = vec![0u8; h.data_size as usize];
        readers.bundle().seek(SeekFrom::Start(h.data_offset))?;
        readers.bundle().read_exact(&mut bundle_buf)?;
    }
    if h.stream_data_size != 0 && u64::from(h.stream_data_offset) < stream_size {
        stream_buf = vec![0u8; h.stream_data_size as usize];
        if readers.stream().is_none() {
            panic!(
                "Stream file referenced but {:?} not found.",
                stream_path.clone()
            );
        }
        if let Some(ref mut sf_reader) = readers.stream() {
            sf_reader.seek(SeekFrom::Start(h.stream_data_offset as u64))?;
            sf_reader.read_exact(&mut stream_buf)?;
        }
    }
    if h.gpu_data_size != 0 && h.gpu_data_offset < gpu_size {
        gpu_buf = vec![0u8; h.gpu_data_size as usize];
        if readers.gpu().is_none() {
            panic!(
                "GPU Resources referenced but {:?} not founh.",
                gpu_path.clone()
            );
        }
        if let Some(ref mut gpu_reader) = readers.gpu() {
            gpu_reader.seek(SeekFrom::Start(h.gpu_data_size as u64))?;
            gpu_reader.read_exact(&mut gpu_buf)?;
        }
    }

    let enum_type: DataTypes = num::FromPrimitive::from_u64(d.type_id.into()).unwrap_or_default();
    let type_folder = format!("{:?}_{:x?}", enum_type, d.type_id);
    out_path = out_path.join(type_folder);
    if namedb.name_database.contains_key(&d.unk_id) {
        out_path = out_path.join(namedb.name_database.get(&d.unk_id).unwrap());
    }
    if !out_path.exists() {
        let _ = std::fs::create_dir_all(&out_path);
    }
    out_path = out_path.join(format!("{}_{}", d.unk4c, d.unk_id));

    if !bundle_buf.is_empty() {
        let mut bundle = out_path.clone();
        bundle.set_extension("bundle.".to_owned() + d.type_enum.extension());
        let mut out_file = File::create(bundle)?;
        out_file.write_all(&bundle_buf)?;
    }
    if !stream_buf.is_empty() {
        let mut stream = out_path.clone();
        stream.set_extension("stream.".to_owned() + d.type_enum.extension());
        let mut out_file = BufWriter::new(File::create(stream)?);
        out_file.write_all(&stream_buf)?;
    }
    if !gpu_buf.is_empty() {
        let mut gpu = out_path.clone();
        gpu.set_extension("gpu.".to_owned() + d.type_enum.extension());
        let mut out_file = File::create(gpu)?;
        out_file.write_all(&gpu_buf)?;
    }

    Ok(())
}

pub fn extract_files(
    cache: &IdCache,
    output_path: &String,
    data_path: &String,
    bundle_file: &String,
    select_type: Option<DataTypes>,
    one_folder: bool,
    namedb: &crate::pndb::Pndb,
) -> anyhow::Result<()> {
    let path = Path::new(data_path).join(bundle_file);
    if !path.exists() {
        panic!("Tried to open nonexistent file {:?}", path);
    }
    // println!("{:?}", path);
    let f = File::open(path)?;

    let bundle_size = f.metadata()?.len();
    let mut reader = BufReader::new(f);

    let header: Header = reader.read_le()?;

    let types: Vec<DataType> = read_types(&mut reader, header)?;
    let mut types_dict: HashMap<Id, &DataType> = Default::default();
    for t in &types {
        types_dict.insert(t.type_id, t);
    }

    let mut data_headers: Vec<DataHeader> = read_data_headers(&mut reader, &types)?;
    let mut stream_path = Path::new(&data_path).join(bundle_file);
    stream_path.set_extension("stream");
    let stream_reader: Option<BufReader<File>>;
    let mut stream_size = 0;

    let mut gpu_path = Path::new(&data_path).join(bundle_file);
    gpu_path.set_extension("gpu_resources");
    let gpu_reader: Option<BufReader<File>>;
    let mut gpu_size = 0;

    let mut readers = DataReaders::new(reader);

    if stream_path.exists() {
        let file = File::open(stream_path.clone())?;
        stream_size = file.metadata()?.len();
        stream_reader = Some(BufReader::new(file));
        readers.set_stream(stream_reader.unwrap())
    }
    if gpu_path.exists() {
        let file = File::open(gpu_path.clone())?;
        gpu_size = file.metadata()?.len();
        gpu_reader = Some(BufReader::new(file));
        readers.set_gpu(gpu_reader.unwrap())
    }

    // println!("{:#?}", types_dict);

    for i in 0..data_headers.len() {
        let d = data_headers.get_mut(i).unwrap();
        // println!("{:#?}", d.unk_id);
        d.type_enum = num::FromPrimitive::from_u64(d.type_id.into()).unwrap_or_default();
        if select_type.is_some() && d.type_enum != select_type.unwrap() {
            continue;
        }
        // let file_ext = match d.type_enum {
        //     DataTypes::WwiseWem => "wem",
        //     DataTypes::WwiseBNK => "bnk",
        //     DataTypes::Havok => "hxt",
        //     DataTypes::Texture => "dds",
        //     DataTypes::Model => "obj",
        //     _ => "bin",
        // };
        let mut out_path = Path::new(output_path).join(bundle_file);
        if one_folder {
            out_path = Path::new(output_path).to_path_buf();
        }
        if export_special(
            cache,
            &Id::from(u64::from_str_radix(bundle_file, 16)?),
            d,
            &mut readers,
            &out_path,
            namedb,
        )? {
            // return Ok(());
            continue;
        }
        let mut bundle_buf: Vec<u8> = Vec::new();
        let mut stream_buf: Vec<u8> = Vec::new();
        let mut gpu_buf: Vec<u8> = Vec::new();
        if d.data_size != 0 {
            bundle_buf = vec![0u8; d.data_size as usize];
            let seek_pos = d.data_offset + types_dict.get(&d.type_id).unwrap().unk10 as u64;
            if seek_pos + d.data_offset >= bundle_size {
                continue;
            }
            readers.bundle().seek(SeekFrom::Start(seek_pos))?;
            readers.bundle().read_exact(&mut bundle_buf)?;
        }
        if d.stream_data_size != 0 && u64::from(d.stream_data_offset) < stream_size {
            stream_buf = vec![0u8; d.stream_data_size as usize];
            if readers.stream().is_none() {
                panic!(
                    "Stream file referenced but {:?} not found.",
                    stream_path.clone()
                );
            }
            if let Some(ref mut sf_reader) = readers.stream() {
                sf_reader.seek(SeekFrom::Start(d.stream_data_offset as u64))?;
                sf_reader.read_exact(&mut stream_buf)?;
            }
        }
        if d.gpu_data_size != 0 && d.gpu_data_offset < gpu_size {
            gpu_buf = vec![0u8; d.gpu_data_size as usize];
            if readers.gpu().is_none() {
                panic!(
                    "GPU Resources referenced but {:?} not found.",
                    gpu_path.clone()
                );
            }
            if let Some(ref mut gpu_reader) = readers.gpu() {
                gpu_reader.seek(SeekFrom::Start(d.gpu_data_size as u64))?;
                gpu_reader.read_exact(&mut gpu_buf)?;
            }
        }

        let enum_type: DataTypes =
            num::FromPrimitive::from_u64(d.type_id.into()).unwrap_or_default();
        let type_folder = format!("{:?}_{:x?}", enum_type, d.type_id);
        out_path = out_path.join(type_folder);
        if namedb.name_database.contains_key(&d.unk_id) {
            out_path = out_path.join(namedb.name_database.get(&d.unk_id).unwrap());
        }
        if !out_path.exists() {
            let _ = std::fs::create_dir_all(&out_path);
        }
        out_path = out_path.join(format!("{}_{}", i, d.unk_id));

        if !bundle_buf.is_empty() {
            let mut bundle = out_path.clone();
            bundle.set_extension("bundle.".to_owned() + d.type_enum.extension());
            let mut out_file = File::create(bundle)?;
            out_file.write_all(&bundle_buf)?;
        }
        if !stream_buf.is_empty() {
            let mut stream = out_path.clone();
            stream.set_extension("stream.".to_owned() + d.type_enum.extension());
            let out_file = File::create(stream);
            if out_file.is_err() {
                println!("{:?}", out_file.err());
                continue;
            }
            let mut out_file = out_file.unwrap();
            out_file.write_all(&stream_buf)?;
        }
        if !gpu_buf.is_empty() {
            let mut gpu = out_path.clone();
            gpu.set_extension("gpu.".to_owned() + d.type_enum.extension());
            let mut out_file = File::create(gpu)?;
            out_file.write_all(&gpu_buf)?;
        }
    }
    Ok(())
}

pub fn read_types(r: &mut BufReader<File>, h: Header) -> anyhow::Result<Vec<DataType>> {
    let mut types: Vec<DataType> = vec![];
    for i in 0..h.type_count {
        let t: DataType = r.read_le()?;
        types.push(t);
        if i < h.type_count - 1 {
            r.seek_relative(8)?;
        }
    }
    Ok(types)
}

pub fn read_data_headers(
    r: &mut BufReader<File>,
    t: &Vec<DataType>,
) -> anyhow::Result<Vec<DataHeader>> {
    let mut headers: Vec<DataHeader> = vec![];
    for type1 in t {
        for _ in 0..type1.data_count {
            let data_header: DataHeader = r.read_le()?;
            headers.push(data_header);
        }
    }
    Ok(headers)
}

pub fn export_special(
    cache: &IdCache,
    bundle_id: &Id,
    d: &mut DataHeader,
    readers: &mut DataReaders,
    out_path: &Path,
    namedb: &crate::pndb::Pndb,
) -> anyhow::Result<bool> {
    let (out_buf, mut file_name) = match d.type_enum {
        DataTypes::Texture => crate::types::texture::extract_texture(d, readers)?,
        DataTypes::Model => crate::types::unit::extract_model(cache, d, readers)?,
        DataTypes::WwiseBNK => crate::types::wwise::extract_bank(cache, bundle_id, d, readers)?,
        DataTypes::WwiseWem => crate::types::wwise::extract_wem(d, readers)?,
        _ => {
            return Ok(false);
        }
    };

    // println!("{:?}", file_name);

    let enum_type: DataTypes = num::FromPrimitive::from_u64(d.type_id.into()).unwrap_or_default();
    let mut out_path = out_path.join(format!("{:?}", enum_type));
    // if !out_path.exists() {
    //     let _ = std::fs::create_dir_all(&out_path);
    // }
    // out_path = out_path.join(format!("{}_{}", d.unk4c, d.unk_id));

    if file_name.is_none() && namedb.name_database.contains_key(&d.unk_id) {
        file_name = namedb.name_database.get(&d.unk_id).cloned();
    }

    let name = if let Some(file_name) = file_name {
        file_name
    } else {
        format!("{}_{}", d.unk4c, d.unk_id)
    };
    // println!("{:?}", name);
    out_path = out_path.join(name);
    out_path.set_extension(d.type_enum.extension());
    // if (out_path.is_dir() && !out_path.exists())
    //     || (out_path.is_file() && !out_path.parent().unwrap().exists())
    // {
    // }
    let p = if out_path.is_dir() {
        out_path.clone()
    } else {
        out_path.parent().unwrap().to_path_buf()
    };
    std::fs::create_dir_all(p)?;

    // println!("{:?}", &out_path);

    let mut out_file = File::create(&out_path)?;
    out_file.write_all(&out_buf)?;

    Ok(true)
}
