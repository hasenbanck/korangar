use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use blake3::Hash;
use hashbrown::HashMap;
#[cfg(feature = "debug")]
use korangar_debug::logging::print_debug;
use korangar_util::container::{SecondarySimpleSlab, SimpleKey};
use korangar_util::texture_atlas::{AllocationId, AtlasAllocation};
use ragnarok_bytes::{ByteReader, ConversionResult, ConversionResultExt, FromBytes, ToBytes};

use crate::loaders::archive::folder::FolderArchive;
use crate::loaders::archive::{Archive, Writable};
use crate::loaders::{GameFileLoader, MapLoader, ModelLoader, TextureAtlasEntry, TextureLoader};

const CACHE_PATH_NAME: &str = "cache";
const MAP_FILE_EXTENSION: &str = ".rsw";

pub struct Cache {
    texture_loader: Arc<TextureLoader>,
    archive: Box<dyn Archive>,
}

impl Cache {
    pub fn new(
        game_file_loader: &GameFileLoader,
        texture_loader: Arc<TextureLoader>,
        map_loader: &MapLoader,
        model_loader: &ModelLoader,
        game_file_hash: Hash,
    ) -> Self {
        let folder_path = Path::new(CACHE_PATH_NAME);
        let file_path = PathBuf::from(format!("{}.grf", CACHE_PATH_NAME));

        // TODO: NHA Verify the hash. If it's not correct, delete the folder/file and
        //       create again
        //
        // TODO: NHA Implement incremental update that verifies every cached file
        let archive: Box<dyn Archive> = if folder_path.exists() && folder_path.is_dir() {
            todo!()
        } else if file_path.exists() && file_path.is_file() {
            todo!()
        } else {
            let hash_string = format!("{:x?}", game_file_hash.as_bytes());
            let mut archive = Box::new(FolderArchive::from_path(Path::new(CACHE_PATH_NAME)));
            archive.add_file_data("game_file_hash.txt", hash_string.as_bytes().to_vec());
            let map_files = game_file_loader.get_files_with_extension(MAP_FILE_EXTENSION);

            #[cfg(feature = "debug")]
            let map_count = map_files.len();

            for (_index, map_file) in map_files.iter().enumerate() {
                let path = Path::new(&map_file);
                let map_name = path.file_stem().unwrap().to_string_lossy().to_string();

                #[cfg(feature = "debug")]
                print_debug!("Creating texture atlas for map `{}`", &map_name);

                match map_loader.generate_texture_atlas(&map_name, model_loader, texture_loader.clone()) {
                    Ok(cached_texture_atlas) => {
                        let atlas_file_path = Self::get_texture_atlas_cache_base_path(&map_name, true, true);
                        let data = cached_texture_atlas
                            .to_bytes()
                            .expect("can't convert cached texture atlas data to bytes");

                        archive.add_file_data(&atlas_file_path, data);
                    }
                    Err(_err) => {
                        #[cfg(feature = "debug")]
                        print_debug!("Error while creating texture atlas for map `{}`: {:?}", map_name, _err);
                    }
                };

                #[cfg(feature = "debug")]
                print_debug!("Finished {} of {} total maps", _index + 1, map_count);
            }

            #[cfg(feature = "debug")]
            print_debug!("Converting to GRF archive");

            let native_archive = archive.save_as_native_archive(&file_path);

            Box::new(native_archive)
        };

        Self { texture_loader, archive }
    }

    fn get_texture_atlas_cache_base_path(name: &str, add_padding: bool, create_mip_map: bool) -> String {
        format!(
            "atlas/{}_{}_{}.dat",
            name,
            if add_padding { "padded" } else { "unpadded" },
            if create_mip_map { "mip" } else { "nomip" }
        )
    }

    pub fn load_texture_atlas(&self, name: &str, add_padding: bool, create_mip_map: bool) -> Option<CachedTextureAtlas> {
        let data_path = Self::get_texture_atlas_cache_base_path(name, add_padding, create_mip_map);
        let data = fs::read(&data_path).ok()?;
        let mut byte_reader = ByteReader::<()>::without_metadata(&data);
        let cached_atlas = CachedTextureAtlas::from_bytes(&mut byte_reader).ok()?;

        Some(cached_atlas)
    }
}

pub struct CachedTextureAtlas {
    pub lookup: HashMap<String, TextureAtlasEntry>,
    pub allocations: SecondarySimpleSlab<AllocationId, AtlasAllocation>,
    pub image: CachedTextureAtlasImage,
}

impl FromBytes for CachedTextureAtlas {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let mut atlas_data = TextureAtlasData::from_bytes(byte_reader).trace::<Self>()?;

        let mut lookup = HashMap::with_capacity(atlas_data.lookup.len());
        atlas_data.lookup.drain(..).for_each(|entry| {
            lookup.insert(entry.name, entry.atlas_entry);
        });

        let mut allocations = SecondarySimpleSlab::with_capacity(atlas_data.allocations.len() as _);

        // It's faster to insert last to front, since we can then allocate all empty
        // slots right from the start.
        atlas_data.allocations.sort_by(|a, b| b.key.cmp(&a.key));

        atlas_data.allocations.drain(..).for_each(|entry| {
            allocations.insert(AllocationId::new(entry.key), entry.atlas_allocation);
        });

        let image = CachedTextureAtlasImage::from_bytes(byte_reader).trace::<Self>()?;

        Ok(CachedTextureAtlas {
            lookup,
            allocations,
            image,
        })
    }
}

impl ToBytes for CachedTextureAtlas {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        let lookup = Vec::from_iter(self.lookup.iter().map(|(name, atlas_entry)| LookupEntry {
            name: name.clone(),
            atlas_entry: *atlas_entry,
        }));
        let allocations = Vec::from_iter(self.allocations.iter().map(|(id, atlas_allocation)| AllocationEntry {
            key: id.key(),
            atlas_allocation: *atlas_allocation,
        }));
        let atlas_data = TextureAtlasData { lookup, allocations };

        let mut bytes = atlas_data.to_bytes().trace::<Self>()?;
        bytes.extend(&self.image.to_bytes().trace::<Self>()?);

        Ok(bytes)
    }
}

#[derive(ToBytes, FromBytes)]
pub struct CachedTextureAtlasImage {
    pub width: u32,
    pub height: u32,
    pub mipmaps_count: u32,
    pub compressed_data: Vec<u8>,
}

#[derive(ToBytes, FromBytes)]
struct TextureAtlasData {
    lookup: Vec<LookupEntry>,
    allocations: Vec<AllocationEntry>,
}

#[derive(ToBytes, FromBytes)]
struct LookupEntry {
    name: String,
    atlas_entry: TextureAtlasEntry,
}

#[derive(ToBytes, FromBytes)]
struct AllocationEntry {
    key: u32,
    atlas_allocation: AtlasAllocation,
}
