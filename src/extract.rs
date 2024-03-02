// use crate::types::unit::*;

use super::structs::*;
use binrw::BinReaderExt;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

pub fn extract_single(
    cache: &IdCache,
    output_path: &String,
    data_path: &String,
    id: Id,
) -> anyhow::Result<()> {
    let (bundle_id, h) = cache.get_by_id(id)?;
    let path = Path::new(data_path).join(bundle_id.to_string());
    if !path.exists() {
        return Err(anyhow::anyhow!("tried to open nonexistent file {:?}", path));
    }
    // println!("{:?}", path);
    let mut reader = BufReader::new(File::open(path)?);
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
    if export_special(
        cache,
        &mut d,
        &mut reader,
        &mut stream_reader,
        &mut gpu_reader,
        &out_path,
    )? {
        return Ok(());
    }

    let mut bundle_buf: Vec<u8> = Vec::new();
    let mut stream_buf: Vec<u8> = Vec::new();
    let mut gpu_buf: Vec<u8> = Vec::new();
    if h.data_size != 0 {
        bundle_buf = vec![0u8; h.data_size as usize];
        reader.seek(SeekFrom::Start(h.data_offset))?;
        reader.read_exact(&mut bundle_buf)?;
    }
    if h.stream_data_size != 0 && u64::from(h.stream_data_offset) < stream_size {
        stream_buf = vec![0u8; h.stream_data_size as usize];
        if stream_reader.is_none() {
            panic!(
                "Stream file referenced but {:?} not found.",
                stream_path.clone()
            );
        }
        if let Some(ref mut sf_reader) = stream_reader {
            sf_reader.seek(SeekFrom::Start(h.stream_data_offset as u64))?;
            sf_reader.read_exact(&mut stream_buf)?;
        }
    }
    if h.gpu_data_size != 0 && h.gpu_data_offset < gpu_size {
        gpu_buf = vec![0u8; h.gpu_data_size as usize];
        if gpu_reader.is_none() {
            panic!(
                "GPU Resources referenced but {:?} not founh.",
                gpu_path.clone()
            );
        }
        if let Some(ref mut gpu_reader) = gpu_reader {
            gpu_reader.seek(SeekFrom::Start(h.gpu_data_size as u64))?;
            gpu_reader.read_exact(&mut gpu_buf)?;
        }
    }

    let enum_type: DataTypes = num::FromPrimitive::from_u64(d.type_id.into()).unwrap_or_default();
    let type_folder = format!("{:?}_{:x?}", enum_type, d.type_id);
    out_path = out_path.join(type_folder);
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

    let mut stream_reader: Option<BufReader<File>> = None;
    let mut stream_size = 0;
    if stream_path.exists() {
        let file = File::open(stream_path.clone())?;
        stream_size = file.metadata()?.len();
        stream_reader = Some(BufReader::new(file));
    }

    let mut gpu_path = Path::new(&data_path).join(bundle_file);
    gpu_path.set_extension("gpu_resources");

    let mut gpu_reader: Option<BufReader<File>> = None;
    let mut gpu_size = 0;
    if gpu_path.exists() {
        let file = File::open(gpu_path.clone())?;
        gpu_size = file.metadata()?.len();
        gpu_reader = Some(BufReader::new(file));
    }
    // println!("{:#?}", types_dict);

    for i in 0..data_headers.len() {
        let d = data_headers.get_mut(i).unwrap();
        println!("{:#?}", d.unk_id);
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
            d,
            &mut reader,
            &mut stream_reader,
            &mut gpu_reader,
            &out_path,
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
            reader.seek(SeekFrom::Start(seek_pos))?;
            reader.read_exact(&mut bundle_buf)?;
        }
        if d.stream_data_size != 0 && u64::from(d.stream_data_offset) < stream_size {
            stream_buf = vec![0u8; d.stream_data_size as usize];
            if stream_reader.is_none() {
                panic!(
                    "Stream file referenced but {:?} not found.",
                    stream_path.clone()
                );
            }
            if let Some(ref mut sf_reader) = stream_reader {
                sf_reader.seek(SeekFrom::Start(d.stream_data_offset as u64))?;
                sf_reader.read_exact(&mut stream_buf)?;
            }
        }
        if d.gpu_data_size != 0 && d.gpu_data_offset < gpu_size {
            gpu_buf = vec![0u8; d.gpu_data_size as usize];
            if gpu_reader.is_none() {
                panic!(
                    "GPU Resources referenced but {:?} not found.",
                    gpu_path.clone()
                );
            }
            if let Some(ref mut gpu_reader) = gpu_reader {
                gpu_reader.seek(SeekFrom::Start(d.gpu_data_size as u64))?;
                gpu_reader.read_exact(&mut gpu_buf)?;
            }
        }

        let enum_type: DataTypes =
            num::FromPrimitive::from_u64(d.type_id.into()).unwrap_or_default();
        let type_folder = format!("{:?}_{:x?}", enum_type, d.type_id);
        out_path = out_path.join(type_folder);
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
    d: &mut DataHeader,
    r: &mut BufReader<File>,
    sf: &mut Option<BufReader<File>>,
    gf: &mut Option<BufReader<File>>,
    out_path: &Path,
) -> anyhow::Result<bool> {
    let (out_buf, file_name) = match d.type_enum {
        DataTypes::Texture => export_texture(d, r, sf, gf)?,
        DataTypes::Model => export_model(cache, d, r, gf)?,
        DataTypes::WwiseBNK => crate::types::wwise::extract_bank(d, r, sf)?,
        DataTypes::WwiseWem => crate::types::wwise::extract_wem(d, r, sf)?,
        _ => {
            return Ok(false);
        }
    };

    let enum_type: DataTypes = num::FromPrimitive::from_u64(d.type_id.into()).unwrap_or_default();
    let mut out_path = out_path.join(format!("{:?}", enum_type));
    if !out_path.exists() {
        let _ = std::fs::create_dir_all(&out_path);
    }
    // out_path = out_path.join(format!("{}_{}", d.unk4c, d.unk_id));
    let name = if let Some(file_name) = file_name {file_name} else {format!("{}_{}", d.unk4c, d.unk_id) };
    out_path = out_path.join(name);
    out_path.set_extension(d.type_enum.extension());

    let mut out_file = File::create(out_path)?;
    out_file.write_all(&out_buf)?;

    Ok(true)
}

fn export_texture(
    d: &mut DataHeader,
    r: &mut BufReader<File>,
    sf: &mut Option<BufReader<File>>,
    gf: &mut Option<BufReader<File>>,
) -> Result<(Vec<u8>, Option<String>), anyhow::Error> {
    let mut out_buf: Vec<u8> = Vec::new();
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

fn export_model(
    cache: &IdCache,
    d: &mut DataHeader,
    r: &mut BufReader<File>,
    gf: &mut Option<BufReader<File>>,
) -> Result<(Vec<u8>, Option<String>), anyhow::Error> {
    let mut out_buf: Vec<u8> = Vec::new();
    r.seek(SeekFrom::Start(d.data_offset))?;
    let mut data = vec![0u8; d.data_size.try_into().unwrap()];
    r.read_exact(&mut data)?;
    let mut mr = BufReader::new(Cursor::new(data));
    let mh: UnitHeader = mr.read_le()?;
    let mut mesh: Mesh = Default::default();
    println!("{:#?}", mh);
    for i in 0..mh.part_count {
        let mut sub_parts: HashMap<u32, PartDef> = Default::default();

        let mut off = mh.part_offset + 4 + i * 4;
        mr.seek(SeekFrom::Start(off.into()))?;

        let rel_off = mr.read_le::<u32>()?;
        off = mh.part_offset + rel_off + 0x3C;

        mr.seek(SeekFrom::Start(off.into()))?;
        let mesh_index = mr.read_le::<i32>()?;
        mr.seek(SeekFrom::Current(0x38))?;
        let count = mr.read_le::<u32>()?;
        mr.seek_relative(4)?;

        let mut ids: Vec<u32> = Vec::new();
        for _ in 0..count {
            ids.push(mr.read_le()?);
        }

        for _ in 0..count {
            let def: PartDef = mr.read_le()?;
            // println!("{:#?}", def);
            sub_parts.insert(*ids.get(def.index as usize).unwrap_or(&u32::MAX), def);
        }
        mesh.parts.entry(mesh_index).or_default();
        for subpart in sub_parts {
            mesh.parts.entry(mesh_index).or_default().push(Part {
                id: subpart.0,
                def: subpart.1,
                material_id: *mh.materials.get(&subpart.0).unwrap_or(&Id::invalid()),
            });
        }
    }
    for i in 0..mh.offsets.len() {
        // println!("{:?}", i);
        let off = mh.lod_offset + mh.offsets.get(i).unwrap() + 0x160;
        mr.seek(SeekFrom::Start(off.into()))?;

        let ml: MeshLod = mr.read_le()?;
        println!("{:#?}", ml);

        // let size = ml.vtx_count * ml.stride as u32;
        let mut data = vec![0u8; ml.vtx_size as usize];
        if gf.is_none() {
            panic!("GPU Resource file referenced but not found.");
        }
        if let Some(ref mut gf) = gf {
            gf.seek(SeekFrom::Start(d.gpu_data_offset + ml.vtx_offset as u64))?;
            gf.read_exact(&mut data)?;
        }
        let mut gr = BufReader::new(Cursor::new(data));
        for _ in 0..ml.vtx_count {
            let mut vtx: Vertex = Default::default();
            // let mut vpos = Vec::new();
            // let mut vt = Vec::new();
            // TODO: this is no good and bad. figure out what Diver does (something like Hellextractor's method?)
            match ml.stride {
                16 => {
                    let pos: Vector3 = gr.read_le()?;
                    let uv: HalfVector2 = gr.read_le()?;
                    vtx.pos = pos;
                    vtx.uv = uv.into();
                }
                20 => {
                    let pos: Vector3 = gr.read_le()?;
                    gr.seek_relative(0x4)?;
                    let uv: HalfVector2 = gr.read_le()?;
                    vtx.pos = pos;
                    vtx.uv = uv.into();
                    // vpos.push(pos);
                    // vt.push(uv);
                }
                24 => {
                    let pos: Vector3 = gr.read_le()?;
                    gr.seek_relative(0x4)?;
                    let uv: HalfVector2 = gr.read_le()?;
                    let _uv2: HalfVector2 = gr.read_le()?;
                    vtx.pos = pos;
                    vtx.uv = uv.into();
                    // vpos.push(pos);
                    // vt.push(uv);
                }
                28 => {
                    let pos: Vector3 = gr.read_le()?;
                    gr.seek_relative(0x4)?;
                    let uv: HalfVector2 = gr.read_le()?;
                    // this just has a single float?
                    vtx.pos = pos;
                    vtx.uv = uv.into();
                    // vpos.push(pos);
                    // vt.push(uv);
                    gr.seek_relative(0x8)?;
                }
                32 => {
                    gr.seek_relative(0x4)?;
                    let pos: Vector3 = gr.read_le()?;
                    let uv: HalfVector2 = gr.read_le()?;
                    let _col: Vector3 = gr.read_le()?;
                    vtx.pos = pos;
                    vtx.uv = uv.into();
                    //vtx.norm = norm;
                }
                36 => {
                    let pos: Vector3 = gr.read_le()?;
                    gr.seek_relative(0x4)?;
                    let uv: HalfVector2 = gr.read_le()?;
                    let _col: HalfVector3 = gr.read_le()?; // looks like vc, but is horribly not accurate colors??
                    vtx.pos = pos;
                    vtx.uv = uv.into();
                    // vtx.norm = norm.into();
                    gr.seek_relative(0xA)?;
                }
                40 => {
                    gr.seek_relative(0x4)?;
                    let pos: Vector3 = gr.read_le()?;
                    gr.seek_relative(0x4)?;
                    let uv: HalfVector2 = gr.read_le()?;
                    let _uv2: HalfVector2 = gr.read_le()?;
                    // gr.seek_relative(0x4)?;
                    // guessing, not right at all i assume
                    let norm: HalfVector3 = gr.read_le()?;
                    vtx.pos = pos;
                    vtx.uv = uv.into();
                    vtx.norm = norm.into();
                }
                48 => {
                    let pos: Vector3 = gr.read_le()?;
                    gr.seek_relative(0x4)?;
                    let uv: Vector2 = gr.read_le()?;
                    gr.seek_relative(0x18)?;
                    vtx.pos = pos;
                    vtx.uv = uv;
                }
                60 => {
                    let pos: Vector3 = gr.read_le()?;
                    gr.seek_relative(0x4)?;
                    let uv: HalfVector2 = gr.read_le()?;
                    // todo: re-check these
                    let _uv2: HalfVector2 = gr.read_le()?;
                    let _col: HalfVector3 = gr.read_le()?; // actually vertex color, where the fuck are normals???
                    vtx.pos = pos;
                    vtx.uv = uv.into();
                    // vtx.norm = norm.into();
                    // vtx.col = col.into();
                    gr.seek_relative(0x1E)?;
                }
                _ => {}
            }
            mesh.vertices.push(vtx);
        }

        let mut data = vec![0u8; ml.idx_size as usize];
        if gf.is_none() {
            panic!("GPU Resource file referenced but not found.");
        }
        if let Some(ref mut gf) = gf {
            gf.seek(SeekFrom::Start(d.gpu_data_offset + ml.idx_offset as u64))?;
            gf.read_exact(&mut data)?;
        }
        let mut gr = BufReader::new(Cursor::new(data));
        let idx_stride = ml.idx_size / ml.idx_count;
        // println!("{:#?} {:?}", ml, idx_count);
        // for _ in 0..idx_count {
        //     // let mut dat = gr.read_le::<[u16;3]>()?.to_vec();
        //     mesh.indices.push(gr.read_le::<[u16; 3]>()?.to_vec());
        // }
        // println!("{:x?}: {:#?}", d.unk_id, mh);

        // if !mesh.parts.contains_key(i) {
        //     continue;
        // }
        let part_defs = mesh.parts.get(&(i as i32)).unwrap();
        let mut a = 0;
        // let mut attrs_writer = fbx_writer.new_node("vertices").unwrap();
        // for vert in mesh.vert_pos.clone().into_iter() {
        //     attrs_writer.append_arr_f32_from_iter(ArrayAttributeEncoding::Direct, vert.into_iter()).unwrap();
        // }
        // fbx_writer.close_node().unwrap();
        for vert in mesh.vertices.iter() {
            // let vert = mh.vertices.get(i).unwrap();
            a += 1;
            out_buf.write_all(format!("i {:?}\n", a).as_bytes())?;
            out_buf.write_all(
                format!("v {:?} {:?} {:?}\n", vert.pos.x, vert.pos.y, vert.pos.z).as_bytes(),
            )?;
            out_buf.write_all(format!("vt {:?} {:?}\n", vert.uv.x, vert.uv.y).as_bytes())?;
            out_buf.write_all(
                format!("vn {:?} {:?} {:?}\n", vert.norm.x, vert.norm.y, vert.norm.z).as_bytes(),
            )?;
        }
        // part code from https://github.com/MontagueM/helldivers2
        // dont know if its functioning for everything yet, helmet model is messed up (8C12FFEFB4D020BC)

        // out_buf.write_all(format!("o {:?}_{}_{:x?}\n", d.unk4c, d.unk_id, i).as_bytes())?;
        // println!("o {:?}_{}_{:x?}", d.unk4c, d.unk_id, i);

        for part in part_defs {
            // println!("{:#?}", part);
            out_buf
                .write_all(format!("o {:?}_{}_{:x?}\n", d.unk4c, d.unk_id, part.id).as_bytes())?;
            let mut local_indices: Vec<Vec<u64>> = Default::default();
            for a in (0..part.def.idx_count).step_by(3) {
                // for a in (0..ml.idx_count).step_by(3) {
                let b = a * idx_stride as i32;
                gr.seek(SeekFrom::Start(b as u64))?;
                // let mut vec1 = gr.read_le::<[u16; 3]>()?.to_vec();
                let mut vec1: Vec<u64> = Vec::new();
                match idx_stride {
                    1 => {
                        vec1 = gr.read_le::<[u8; 3]>()?.iter().map(|x| *x as u64).collect();
                    }
                    2 => {
                        vec1 = gr
                            .read_le::<[u16; 3]>()?
                            .iter()
                            .map(|x| *x as u64)
                            .collect();
                    }
                    4 => {
                        vec1 = gr
                            .read_le::<[u32; 3]>()?
                            .iter()
                            .map(|x| *x as u64)
                            .collect();
                    }
                    8 => {
                        vec1 = gr.read_le::<[u64; 3]>()?.to_vec();
                    }
                    _ => {}
                }
                let mut vec2 = Vec::new();
                for b in vec1.iter_mut() {
                    // vec2.push(*b + ml.vtx_offset as u64);
                    vec2.push(*b + part.def.vtx_offset as u64);
                }
                local_indices.push(vec2);
            }

            for idx in local_indices {
                out_buf.write_all(
                    format!(
                        "f {:?}/{:?}/{:?} {:?}/{:?}/{:?} {:?}/{:?}/{:?}\n",
                        idx[0] + 1,
                        idx[0] + 1,
                        idx[0] + 1,
                        idx[1] + 1,
                        idx[1] + 1,
                        idx[1] + 1,
                        idx[2] + 1,
                        idx[2] + 1,
                        idx[2] + 1
                    )
                    .as_bytes(),
                )?;
            }
        }
        // TODO: materials + textures
        // for part in part_defs {
        //     if part.material_id == Id::invalid() {
        //         continue;
        //     }
        //     if let Ok((mat_bundle, mat_head)) = cache.get_by_id(part.material_id) {

        //     }
        // }
    }
    Ok((out_buf, None))
}
