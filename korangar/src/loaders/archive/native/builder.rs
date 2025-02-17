//! Implements a writable instance of a GRF File
//! This way, we can provide a temporal storage to files before the final write
//! occurs while keeping it outside the
//! [`NativeArchive`](super::NativeArchive) implementation

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::bufread::ZlibEncoder;
use flate2::Compression;
use ragnarok_bytes::{FixedByteSize, ToBytes};
use ragnarok_formats::archive::{AssetTable, FileTableRow, Header};

use super::FileTable;
use crate::loaders::archive::Writable;

struct FileTableEntry {
    path: String,
    data: ArchiveEntry,
}

enum ArchiveEntry {
    Data(Vec<u8>),
    File(PathBuf),
}

pub struct NativeArchiveBuilder {
    os_file_path: PathBuf,
    archive_entries: Vec<FileTableEntry>,
}

impl NativeArchiveBuilder {
    pub fn from_path(path: &Path) -> Self {
        Self {
            os_file_path: PathBuf::from(path),
            archive_entries: Vec::new(),
        }
    }
}

impl Writable for NativeArchiveBuilder {
    fn add_file(&mut self, path: &str, os_file_path: &Path) {
        self.archive_entries.push(FileTableEntry {
            path: path.to_string(),
            data: ArchiveEntry::File(os_file_path.to_path_buf()),
        });
    }

    fn add_file_data(&mut self, path: &str, asset: Vec<u8>) {
        self.archive_entries.push(FileTableEntry {
            path: path.to_string(),
            data: ArchiveEntry::Data(asset),
        });
    }

    fn save(&mut self) {
        let mut file_table = FileTable::new();

        let file_header_size = Header::size_in_bytes();
        let dummy_file_header = Header::new(42, 0, 0, 0);
        let mut bytes = dummy_file_header.to_bytes().unwrap();

        for entry in self.archive_entries.drain(..) {
            match entry.data {
                ArchiveEntry::Data(data) => {
                    add_asset_to_file_table(&mut bytes, &mut file_table, &entry.path, &data);
                }
                ArchiveEntry::File(path) => {
                    let data = fs::read(path).expect("can't read file to archive");
                    add_asset_to_file_table(&mut bytes, &mut file_table, &entry.path, &data);
                }
            }
        }

        let mut file_table_data = Vec::new();

        for file_information in file_table.values() {
            file_table_data.extend(file_information.to_bytes().unwrap());
        }

        let mut encoder = ZlibEncoder::new(file_table_data.as_slice(), Compression::best());
        let mut compressed = Vec::default();
        encoder.read_to_end(&mut compressed).expect("can't compress file information");

        let asset_table = AssetTable {
            compressed_size: compressed.len() as u32,
            uncompressed_size: file_table_data.len() as u32,
        };

        let file_table_offset = bytes.len();
        bytes.extend(asset_table.to_bytes().unwrap());
        bytes.extend(compressed);

        let file_table_offset = (file_table_offset - file_header_size) as u32;
        let reserved_files = 0;
        let raw_file_count = file_table.len() as u32 + 7;
        let version = 0x200;
        let file_header_bytes = Header::new(file_table_offset, reserved_files, raw_file_count, version)
            .to_bytes()
            .unwrap();
        bytes[..file_header_size].copy_from_slice(&file_header_bytes[..file_header_size]);

        fs::write(&self.os_file_path, bytes).expect("unable to write file");
    }
}

fn add_asset_to_file_table(bytes: &mut Vec<u8>, file_table: &mut HashMap<String, FileTableRow>, path: &str, data: &[u8]) {
    let mut encoder = ZlibEncoder::new(data, Compression::best());
    let mut compressed = Vec::default();
    encoder.read_to_end(&mut compressed).expect("can't compress asset data");

    let compressed_size = compressed.len() as u32;
    let compressed_size_aligned = compressed_size;
    let uncompressed_size = data.len() as u32;
    let flags = 1;

    let file_information = FileTableRow {
        file_name: path.to_string(),
        compressed_size,
        compressed_size_aligned,
        uncompressed_size,
        flags,
        offset: (bytes.len() - Header::size_in_bytes()) as u32,
    };

    file_table.insert(path.to_string(), file_information);
    bytes.extend(compressed);
}
