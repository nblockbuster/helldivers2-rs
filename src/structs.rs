use std::{collections::HashMap, f32::consts::PI, io::SeekFrom};

use binrw::{binread, binrw, BinRead, BinWrite};
use serde::{Deserialize, Serialize};

#[derive(clap::ValueEnum, BinRead, Debug, Default, FromPrimitive, ToPrimitive, PartialEq, Clone, Copy)]
#[br(repr = u64)]
#[repr(u64)]
pub enum DataTypes {
    #[default]
    #[value(name = "unknown")]
    Unknown = 0xFFFFFFFF_FFFFFFFF,

    #[value(name = "wem")]
    WwiseWem = 0xE44215D_23554B50,

    #[value(name = "bnk")]
    WwiseBNK = 0x99D750E6_D37B5A53,

    #[value(name = "havok1")]
    Havok = 0xb8da80f2_c803725f,

    #[value(name = "havok2")]
    Havok2 = 0x336bdb87_66bd591d,

    #[value(name = "texture")]
    Texture = 0x329ec6a0_c63842cd,

    #[value(name = "model")]
    Model = 0x3f45a7e9_0b8da4e0,

    #[value(name = "string")]
    String = 0xd30fb410_ab2b970d,

    #[value(name = "entity")]
    Entity = 0x7d080d3b_89ca3198,

    #[value(name = "material")]
    Material = 0xDFDE6A87_97B4C0EA,
}

impl DataTypes {
    pub fn extension(self) -> &'static str {
        match self {
            DataTypes::WwiseWem => "wem",
            DataTypes::WwiseBNK => "bnk",
            DataTypes::Havok |
            DataTypes::Havok2 => "hkt",
            DataTypes::Texture => "dds",
            DataTypes::Model => "obj",
            _ => "bin"
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct IdCache {
    pub bundles: HashMap<Id, Vec<MinimizedIdHeader>>
}

impl BinRead for IdCache {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        (): Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let mut bundles = HashMap::new();
        let bundle_count = u32::read_options(reader, endian, ())?;
        for _ in 0..bundle_count {
            let id = Id::read_options(reader, endian, ())?;
            let header_count = u32::read_options(reader, endian, ())?;
            let mut headers = Vec::new();
            for _ in 0..header_count {
                let header = MinimizedIdHeader::read_options(reader, endian, ())?;
                headers.push(header);
            }
            bundles.insert(id, headers);
        }
        Ok(IdCache {
            bundles
        })
    }
}

impl BinWrite for IdCache {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        (): Self::Args<'_>,
    ) -> binrw::BinResult<()> {
        let bundle_count = self.bundles.len() as u32;
        bundle_count.write_options(writer, endian, ())?;
        for (id, headers) in &self.bundles {
            id.write_options(writer, endian, ())?;
            let header_count = headers.len() as u32;
            header_count.write_options(writer, endian, ())?;
            for header in headers {
                header.write_options(writer, endian, ())?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[binrw(little)]
pub struct MinimizedIdHeader {
    // #[serde(skip)]
    pub id: Id,
    // #[brw(ignore)]
    // #[serde(rename = "id")]
    // pub _id: String,
    // #[serde(skip)]
    pub type_id: Id,
    // #[brw(ignore)]
    // #[serde(rename = "type_id")]
    // pub _type_id: String,
    pub data_offset: u64,
    pub data_size: u32,
    pub stream_data_offset: u32,
    pub stream_data_size: u32,
    pub gpu_data_offset: u64,
    pub gpu_data_size: u32,
}

// u64, binrw reads as u64, serde reads as string, both output to string (binrw: hex u64, serde: string)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Id {
    _id: u64
}

impl Id {
    pub fn new(id: u64) -> Self {
        Id {
            _id: id
        }
    }
}

impl From<u64> for Id {
    fn from(id: u64) -> Self {
        Id::new(id)
    }
}

impl From<Id> for u64 {
    fn from(id: Id) -> Self {
        id._id
    }
}

impl From<Id> for String {
    fn from(id: Id) -> Self {
        format!("{:x}", id._id)
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Id::new(u64::from_str_radix(s, 16).unwrap())
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Id::new(u64::from_str_radix(&s, 16).unwrap())
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", &self._id)
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", &self._id)
    }
}

impl serde::Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        serializer.serialize_str(&format!("{:x}", &self._id))
    }
}

impl<'de> serde::Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Id, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        let id = u64::from_str_radix(&s, 16).unwrap();
        Ok(Id::new(id))
    }
}

impl BinRead for Id {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        (): Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let id = u64::read_options(reader, endian, ())?;
        Ok(Id::new(id))
    }
}

impl BinWrite for Id {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        (): Self::Args<'_>,
    ) -> binrw::BinResult<()> {
        self._id.write_options(writer, endian, ())
    }
}

#[derive(Debug)]
pub struct Bundle {
    pub header: Header,
    pub data_types: Vec<DataType>,
    pub data_headers: Vec<DataHeader>,
}

#[derive(BinRead, Debug)]
#[br(magic = 0xF0000011_u32)]
// Bundle files
pub struct Header {
    pub type_count: u32,
    pub data_count: u64,
    pub unk10: u32,
    pub unk14: u32,
    pub unk18: u64,
    pub unk20: u64,
    pub unk28: u64,
    pub unk30_pad: (u128, u128),
}

#[derive(Debug)]
#[binread]
pub struct DataType {
    #[br(big)]
    pub type_id: u64,
    pub data_count: u64,
    pub unk10: u32, // offset this much when reading in data file and not stream?
    pub unk14: u32,
}

#[derive(BinRead, Debug, Default, Clone, Copy)]
pub struct DataHeader {
    // #[br(big)]
    pub unk_id: Id,
    // #[br(big)]
    pub type_id: Id,
    pub data_offset: u64,
    pub stream_data_offset: u32,
    pub unk1c: u32,
    pub gpu_data_offset: u64,
    pub unk28: u64,
    pub unk30: u64,
    pub data_size: u32,
    pub stream_data_size: u32,
    pub gpu_data_size: u32,
    pub unk44: u32,
    pub unk48: u32,
    pub unk4c: u32,
    #[br(ignore)]
    pub type_enum: DataTypes,
}

impl From<MinimizedIdHeader> for DataHeader {
    fn from(header: MinimizedIdHeader) -> Self {
        DataHeader {
            unk_id: header.id,
            type_id: header.type_id,
            data_offset: header.data_offset,
            stream_data_offset: header.stream_data_offset,
            gpu_data_offset: header.gpu_data_offset,
            data_size: header.data_size,
            stream_data_size: header.stream_data_size,
            gpu_data_size: header.gpu_data_size,
            ..Default::default()
        }
    }
}

#[derive(BinRead, Debug, Default)]
pub struct ModelHeader {
    #[br(seek_before = SeekFrom::Start(0x5C))]
    pub lod_offset: u32,
    #[br(seek_before = SeekFrom::Start(lod_offset.into()))]
    pub lod_count: u32,

    #[br(count = lod_count)]//, seek_before = SeekFrom::Start(lod_offset as u64 +4))]
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
}

// #[derive(BinRead, Debug, Default)]
// pub struct PartHeader {
//     #[br(seek_before = SeekFrom::Current(0x3C), pad_after = 0x28)]
//     pub mesh_index: i32,
// }

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
    pub material_id: u64,
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
    pub uv: UShortVector2,
    pub norm: Vector3,
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
    // #[br(map = |x: u16| f32::from(x)/65535.0f32)]
    pub x: u16,
    // #[br(map = |y: u16| f32::from(y)/65535.0f32)]
    pub y: u16,
    // #[br(map = |z: u16| f32::from(z)/65535.0f32)]
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
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Debug, Default)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32
}


impl Vector4 {
    pub fn magnitude(s: &Self) -> f32 {
        (s.x * s.x + s.y * s.y + s.z * s.z + s.w * s.w).sqrt()
    }

    pub fn quat_to_euler(s: &Self) -> Vector3 {
        let mut res = Vector3::default();
        if Self::magnitude(s) - 1.0 < 0.01 {
            // roll (x-axis rotation)
            let sinr_cosp = 2.0 * (s.w * s.x + s.y * s.z);
            let cosr_cosp = 1.0 - 2.0 * (s.x * s.x + s.y * s.y);
            res.x = sinr_cosp.atan2(cosr_cosp);

            // pitch (y-axis rotation)
            let sinp = 2.0 * (s.w * s.y - s.z * s.x);
            let abs_sinp = sinp.abs();
            if abs_sinp >= 1.0 {
                res.y = 90.0; // use 90 degrees if out of range
            }
            else
            {
                res.y = sinp.asin();
            }

            // yaw (z-axis rotation)
            let siny_cosp = 2.0 * (s.w * s.z + s.x * s.y);
            let cosy_cosp = 1.0 - 2.0 * (s.y * s.y + s.z * s.z);
            res.z = siny_cosp.atan2(cosy_cosp);

            // Rad to Deg
            res.x *= 180.0 / PI;

            if abs_sinp < 1.0 { // only mult if within range
                res.y *= 180.0 / PI;
            }
            res.z *= 180.0 / PI;
        }
        else {
            res.x = s.x;
            res.y = s.y;
            res.z = s.z;
        }
        res
    }
}