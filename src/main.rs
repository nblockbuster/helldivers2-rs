#![deny(clippy::correctness, clippy::suspicious, clippy::complexity)]
// Uses research and code done by MontagueM at https://github.com/MontagueM/helldivers2,
// as well as from h3x3r and Xaymar at https://reshax.com/topic/507-helldivers-2-model-extraction-help
pub mod extract;
pub mod pndb;
pub mod structs;
pub mod types;

use binrw::{BinReaderExt, BinWriterExt};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::Path,
    time::Instant,
};

use extract::*;
use structs::*;

#[macro_use]
extern crate num_derive;
use clap::Parser;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None, disable_version_flag(true))]
struct Args {
    /// Path to data directory
    data_path: String,

    /// Path to output files to
    output_path: String,

    /// Selected bundle file
    bundle_file: Option<String>,

    /// Extract from all bundles
    #[arg(short, long)]
    extract_all: bool,

    /// Extracts everyting into one folder (still separated by type)
    #[arg(short, long)]
    one_folder: bool,

    /// Rebuilds the ID cache
    #[arg(short, long)]
    build_cache: bool,

    /// Types to extract
    #[arg(short, value_enum)]
    filetype: Option<DataTypes>,

    /// Extract all files with x ID
    #[arg(short, value_enum)]
    selected_id: Option<String>,

    /// Uses an assets.pndb file in the same location as the exe to apply names to files
    #[arg(short, long)]
    pndb: bool,
}

pub fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.build_cache || !Path::new("ids.cache").exists() {
        println!("Building cache...");
        let start = Instant::now();
        let cache = build_id_cache(&args.data_path)?;
        let mut cache_file = File::create("id_cache.json")?;
        let json = serde_json::to_string(&cache)?;
        cache_file.write_all(json.as_bytes())?;

        let mut cache_writer = BufWriter::new(File::create("ids.cache")?);
        cache_writer.write_le(&cache)?;

        let end = Instant::now() - start;
        println!(
            "Done. {:?} bundles with {:?} files saved to ids.cache in {:?}ms.",
            cache.bundles.len(),
            cache.bundles.values().map(|x| x.len()).sum::<usize>(),
            end.as_millis()
        );
        if args.build_cache {
            return Ok(());
        }
    }

    println!("Loading cache...");
    let start = Instant::now();
    let mut reader = BufReader::new(File::open("ids.cache")?);

    let cache: IdCache = reader.read_le()?;

    let end = Instant::now() - start;
    println!(
        "{:?} bundles with {:?} files loaded in {:?}ms.",
        cache.bundles.len(),
        cache.bundles.values().map(|x| x.len()).sum::<usize>(),
        end.as_millis()
    );

    let mut namedb = pndb::Pndb::default();
    if args.pndb {
        namedb = pndb::read_pndb("assets.pndb")?;
    }


    std::fs::create_dir_all(&args.output_path)?;

    if args.selected_id.is_some() {
        // for (bundle, headers) in cache.bundles.iter() {
        //     for header in headers {
        //         if header.id == id {
        //             println!("id {:?} is in bundle {:?}", header.id, bundle);
        //             extract_single(&args.output_path, &args.data_path, bundle, header)?;
        //         }
        //     }
        // }
        // println!("id {:?} is in bundle {:?}", id, data.0);
        return extract_single(
            &cache,
            &args.output_path,
            &args.data_path,
            Id::from(args.selected_id.unwrap()),
            &namedb
        );
    }

    if args.extract_all {
        for a in std::fs::read_dir(&args.data_path)? {
            let bundle_name = a?.file_name();
            let bundle_name = bundle_name.to_str().unwrap();
            if bundle_name.contains('.') || bundle_name == "game" {
                continue;
            }
            extract_files(
                &cache,
                &args.output_path,
                &args.data_path,
                &bundle_name.to_string(),
                args.filetype,
                args.one_folder,
                &namedb
            )?;
        }
        return Ok(());
    } else if args.bundle_file.is_none() {
        println!("You must either select a single bundle or extract all.");
        return Ok(());
    }
    extract_files(
        &cache,
        &args.output_path,
        &args.data_path,
        &args.bundle_file.unwrap(),
        args.filetype,
        args.one_folder,
        &namedb
    )?;

    Ok(())
}

fn build_id_cache(data_path: &String) -> anyhow::Result<IdCache> {
    let mut cache: IdCache = Default::default();

    for a in std::fs::read_dir(data_path)? {
        let bundle_name = a?.file_name();
        let bundle_name = bundle_name.to_str().unwrap();
        if bundle_name.contains('.') || bundle_name == "game" {
            continue;
        }
        let path = Path::new(data_path).join(bundle_name);

        let mut reader = BufReader::new(File::open(path)?);
        let header: Header = reader.read_le()?;
        let types: Vec<DataType> = read_types(&mut reader, header)?;

        let mut types_dict: HashMap<Id, &DataType> = Default::default();
        for t in &types {
            types_dict.insert(t.type_id, t);
        }
        let data_headers: Vec<DataHeader> = read_data_headers(&mut reader, &types)?;

        let mut min_headers: Vec<MinimizedIdHeader> = Vec::new();
        for d in data_headers {
            min_headers.push(MinimizedIdHeader {
                id: d.unk_id,
                type_id: d.type_id,
                data_offset: d.data_offset,
                data_size: d.data_size,
                stream_data_offset: d.stream_data_offset,
                stream_data_size: d.stream_data_size,
                gpu_data_offset: d.gpu_data_offset,
                gpu_data_size: d.gpu_data_size,
            });
        }

        cache.bundles.insert(bundle_name.into(), min_headers);
    }

    Ok(cache)
}
