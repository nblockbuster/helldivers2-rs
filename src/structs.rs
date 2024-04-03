use anyhow::Result;
use binrw::{binread, binrw, BinRead, BinWrite};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
};

#[derive(
    clap::ValueEnum, BinRead, Debug, Default, FromPrimitive, ToPrimitive, PartialEq, Clone, Copy,
)]
#[br(repr = u64)]
#[repr(u64)]
pub enum DataTypes {
    #[default]
    #[value(name = "unknown")]
    Unknown = 0xFFFFFFFF_FFFFFFFF,

    #[value(name = "wem")]
    WwiseWem = 0x504b5523_5d21440e,

    #[value(name = "bnk")]
    WwiseBNK = 0x535A7BD3_E650D799,


    // TODO: re-check these

    // #[value(name = "havok1")]
    // Havok = 0x5F7203C8_F280DAB8,

    // #[value(name = "havok2")]
    // Havok2 = 0x1D59BD66_87DB6B33,

    #[value(name = "texture")]
    Texture = 0xCD4238C6_A0C69E32,

    #[value(name = "unit")]
    Unit = 0xE0A48D0B_E9A7453F,

    #[value(name = "string")]
    String = 0x0D972BAB_10B40FD3,

    // #[value(name = "entity")]
    // Entity = 0x7d080d3b_89ca3198,

    // #[value(name = "material")]
    // Material = 0xDFDE6A87_97B4C0EA,

    // Skeleton = 0xe9726b05_01adde18,

    WwiseDep = 0xAF32095C_82F2B070,
    WwiseMetadata = 0xD50A8B7E_1C82B110,
    WwiseProperties = 0x5FDD5FE3_91076F9F,
}

impl DataTypes {
    pub fn extension(&self) -> &'static str {
        match self {
            DataTypes::WwiseWem => "wem",
            DataTypes::WwiseBNK => "bnk",
            // DataTypes::Havok | DataTypes::Havok2 => "hkt",
            DataTypes::Texture => "dds",
            DataTypes::Unit => "obj",
            DataTypes::String => "json",
            _ => "bin",
        }
    }

    pub fn as_id(&self) -> Id {
        Id::from(num::ToPrimitive::to_u64(self).unwrap())
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct IdCache {
    pub bundles: HashMap<Id, Vec<MinimizedIdHeader>>,
}

impl IdCache {
    pub fn get_by_id(
        &self,
        x: Id,
        t: Option<DataTypes>,
        b: Id
    ) -> anyhow::Result<(Id, MinimizedIdHeader)> {
        if b != Id::invalid() {
            for header in self.bundles.get(&b).unwrap() {
                if header.id == x {
                    if let Some(a) = t {
                        if header.type_id != a.as_id() {
                            continue;
                        }
                    }
                    return Ok((b, *header));
                }
            }
            return Err(anyhow::anyhow!("id {} not found in cache for bundle {}", x, b));
        }
        for (bundle, headers) in self.bundles.iter() {
            for header in headers {
                if header.id == x {
                    if let Some(a) = t {
                        if header.type_id != a.as_id() {
                            continue;
                        }
                    }
                    return Ok((*bundle, *header));
                }
            }
        }
        Err(anyhow::anyhow!("id {} not found in cache", x))
    }
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
            //headers.dedup_by(|a, b| a.id == b.id && a.type_id == b.type_id);
            bundles.insert(id, headers);
        }
        Ok(IdCache { bundles })
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

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
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
    _id: u64,
}

impl Id {
    pub fn new(id: u64) -> Self {
        Id { _id: id }
    }
    pub fn as_enum(&self) -> DataTypes {
        num::FromPrimitive::from_u64(self._id).unwrap_or(DataTypes::Unknown)
    }
    pub fn invalid() -> Self {
        Id { _id: u64::MAX }
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
        format!("{:016x}", id._id)
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
        write!(f, "{:016x}", &self._id)
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", &self._id)
    }
}

impl serde::Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{:016x}", &self._id))
    }
}

impl<'de> serde::Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Id, D::Error>
    where
        D: serde::Deserializer<'de>,
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
        _endian: binrw::Endian,
        (): Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let id = u64::read_options(reader, binrw::Endian::Little, ())?;
        Ok(Id::new(id))
    }
}

impl BinWrite for Id {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _endian: binrw::Endian,
        (): Self::Args<'_>,
    ) -> binrw::BinResult<()> {
        self._id.write_options(writer, binrw::Endian::Little, ())
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
    pub type_id: Id,
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

impl DataHeader {
    pub fn get_stream_buf(&self, r: &mut Option<BufReader<File>>) -> Result<Vec<u8>> {
        if self.stream_data_size == 0 {
            return Err(anyhow::anyhow!("stream data size 0"));
        }
        let mut buf = vec![0u8; self.stream_data_size as usize];
        if r.is_none() {
            panic!("Stream file referenced but not found.");
        }
        if let Some(ref mut reader) = r {
            reader.seek(SeekFrom::Start(self.stream_data_offset as u64))?;
            reader.read_exact(&mut buf)?;
        }

        Ok(buf)
    }

    pub fn get_bundle_buf(&self, r: &mut BufReader<File>) -> Result<Vec<u8>> {
        if self.data_size == 0 {
            return Err(anyhow::anyhow!("bundle data size 0"));
        }
        let mut buf = vec![0u8; self.data_size as usize];
        r.seek(SeekFrom::Start(self.data_offset))?;
        r.read_exact(&mut buf)?;

        Ok(buf)
    }
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
            type_enum: header.type_id.as_enum(),
            ..Default::default()
        }
    }
}

// workaround for E0117
#[derive(Debug, Default)]
pub struct U32IdMap(HashMap<u32, Id>);

impl U32IdMap {
    pub fn get(&self, k: &u32) -> Option<&Id> {
        self.0.get(k)
    }
}

impl BinRead for U32IdMap {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        (): Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let mut materials = HashMap::new();
        let material_count = u32::read_options(reader, endian, ())?;
        for _ in 0..material_count {
            let id = Id::read_options(reader, endian, ())?;
            let index = u32::read_options(reader, endian, ())?;
            materials.insert(index, id);
        }
        Ok(Self(materials))
    }
}

pub struct DataReaders(pub BufReader<File>, pub Option<BufReader<File>>, pub Option<BufReader<File>>);

impl DataReaders {
    pub fn new(r: BufReader<File>) -> Self {
        DataReaders(r, None, None)
    }

    pub fn bundle(&mut self) -> &mut BufReader<File> {
        &mut self.0
    }
    pub fn stream(&mut self) -> &mut Option<BufReader<File>> {
        &mut self.1
    }
    pub fn gpu(&mut self) -> &mut Option<BufReader<File>> {
        &mut self.2
    }

    pub fn set_stream(&mut self, r: BufReader<File>) {
        self.1 = Some(r);
    }
    pub fn set_gpu(&mut self, r: BufReader<File>) {
        self.2 = Some(r);
    }
}