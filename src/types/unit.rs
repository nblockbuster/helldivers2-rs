use std::{collections::HashMap, io::{BufReader, Cursor, Read, Seek, SeekFrom, Write}};

use binrw::{BinRead, BinReaderExt};
use half::f16;

use crate::{DataHeader, DataReaders, Id, IdCache, U32IdMap};

pub fn extract_model(
    _cache: &IdCache,
    d: &mut DataHeader,
    readers: &mut DataReaders
) -> Result<(Vec<u8>, Option<String>), anyhow::Error> {
    let mut out_buf: Vec<u8> = Vec::new();
    let r = readers.bundle();
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
        let gf = readers.gpu();
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

#[derive(BinRead, Debug, Default)]
pub struct UnitHeader {
    #[br(seek_before = SeekFrom::Start(0x5C))]
    pub lod_offset: u32,
    #[br(seek_before = SeekFrom::Start(lod_offset.into()))]
    pub lod_count: u32,

    #[br(count = lod_count)] //, seek_before = SeekFrom::Start(lod_offset as u64 +4))]
    pub offsets: Vec<u32>,

    // #[br(seek_before = SeekFrom::Start(0x70))]
    // pub material_offset: u32
    #[br(seek_before = SeekFrom::Start(0x64))]
    pub part_offset: u32,
    #[br(seek_before = SeekFrom::Start(part_offset.into()))]
    pub part_count: u32,
    // #[br(count = part_count)]
    // #[br(ignore)]
    // pub part_indices: Vec<u32>,
    #[br(seek_before = SeekFrom::Start(0x64))]
    pub material_offset: u32,
    // pub material_count: u32,

    // #[br(ignore)]
    #[br(seek_before = SeekFrom::Start(material_offset.into()))]
    pub materials: U32IdMap,
}

#[derive(Default)]
pub struct Mesh {
    // pub header: ModelHeader,
    pub parts: HashMap<i32, Vec<Part>>,
    pub vertices: Vec<Vertex>,
    // pub vert_pos: Vec<Vec<f32>>,
    // pub vert_uv: Vec<Vec<f32>>,
    // pub vert_norm: Vec<Vec<f32>>,
    pub indices: Vec<Vec<u16>>,
}

#[derive(BinRead, Default, Debug)]
pub struct PartDef {
    pub index: i32,
    pub vtx_offset: i32,
    pub vtx_count: i32,
    pub idx_offset: i32,
    #[br(pad_after = 0x4)]
    pub idx_count: i32,
}

#[derive(BinRead, Default, Debug)]
pub struct Part {
    pub id: u32,
    pub def: PartDef,
    pub material_id: Id,
}

#[derive(BinRead, Debug)]
pub struct MeshLod {
    pub vtx_count: u32,
    pub stride: i32,
    #[br(seek_before = SeekFrom::Current(0x20))]
    pub idx_count: u32,
    #[br(seek_before = SeekFrom::Current(0x14))]
    pub vtx_offset: u32,
    pub vtx_size: u32,
    pub idx_offset: u32,
    pub idx_size: u32,
}

#[derive(Debug, Default)]
pub struct Vertex {
    pub pos: Vector3,
    pub uv: Vector2,
    pub norm: Vector3,
    pub col: Vector3,
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Debug, Default)]
pub struct UShortVector2 {
    #[br(map = |x: u16| f32::from(x)/65535.0f32)]
    pub x: f32,
    #[br(map = |y: u16| f32::from(y)/65535.0f32)]
    pub y: f32,
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Debug, Default)]
pub struct UShortVector3 {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

impl From<UShortVector3> for Vector3 {
    fn from(v: UShortVector3) -> Self {
        Vector3 {
            x: f32::from(v.x) / 65535.0,
            y: f32::from(v.y) / 65535.0,
            z: f32::from(v.z) / 65535.0,
        }
    }
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Debug, Default)]
pub struct HalfVector2 {
    #[br(map = |x: u16| f16::from_bits(x))]
    pub x: f16,
    #[br(map = |x: u16| f16::from_bits(x))]
    pub y: f16,
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Debug, Default)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl From<HalfVector2> for Vector2 {
    fn from(v: HalfVector2) -> Self {
        Vector2 {
            x: f32::from(v.x),
            y: f32::from(v.y),
        }
    }
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Debug, Default)]
pub struct U8Vector3 {
    #[br(map = |x: u8| f32::from(x)/255.0)]
    pub x: f32,
    #[br(map = |x: u8| f32::from(x)/255.0)]
    pub y: f32,
    #[br(map = |x: u8| f32::from(x)/255.0)]
    pub z: f32,
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Debug, Default)]
pub struct HalfVector3 {
    #[br(map = |x: u16| f16::from_bits(x))]
    pub x: f16,
    #[br(map = |x: u16| f16::from_bits(x))]
    pub y: f16,
    #[br(map = |x: u16| f16::from_bits(x))]
    pub z: f16,
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Debug, Default)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl From<HalfVector3> for Vector3 {
    fn from(v: HalfVector3) -> Self {
        Vector3 {
            x: f32::from(v.x),
            y: f32::from(v.y),
            z: f32::from(v.z),
        }
    }
}
