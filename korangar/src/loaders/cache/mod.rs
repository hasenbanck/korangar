use std::path::Path;
use std::sync::Arc;

use blake3::Hash;
use hashbrown::{HashMap, HashSet};
#[cfg(feature = "debug")]
use korangar_debug::logging::print_debug;
use korangar_util::container::{SecondarySimpleSlab, SimpleKey};
use korangar_util::texture_atlas::{AllocationId, AtlasAllocation};
use ragnarok_bytes::{ByteReader, ConversionResult, ConversionResultExt, FromBytes, ToBytes};

use crate::loaders::archive::folder::FolderArchive;
use crate::loaders::archive::{Archive, Writable};
use crate::loaders::{GameFileLoader, MapLoader, ModelLoader, TextureAtlas, TextureAtlasEntry, TextureLoader, UncompressedTextureAtlas};

const HASH_FILE_PATH: &str = "game_file_hash.txt";
const CACHE_PATH_NAME: &str = "cache";
const MAP_FILE_EXTENSION: &str = ".rsw";

// TODO: NHA Implement incremental update that verifies every cached file
pub struct Cache {
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
        let archive = Self::get_cache_archive(game_file_loader, texture_loader, map_loader, model_loader, game_file_hash);

        Self { archive }
    }

    fn get_cache_archive(
        game_file_loader: &GameFileLoader,
        texture_loader: Arc<TextureLoader>,
        map_loader: &MapLoader,
        model_loader: &ModelLoader,
        game_file_hash: Hash,
    ) -> Box<dyn Archive> {
        let folder_path = Path::new(CACHE_PATH_NAME);

        if !folder_path.exists() && !folder_path.is_dir() {
            return Self::create_new_cache_folder(game_file_loader, texture_loader, map_loader, model_loader, game_file_hash);
        }

        let folder_archive = FolderArchive::from_path(folder_path);

        let Some(hash_file) = folder_archive.get_file_by_path(HASH_FILE_PATH) else {
            #[cfg(feature = "debug")]
            print_debug!("Can't find game hash file. Deleting folder and create new cache");
            std::fs::remove_dir_all(folder_path).ok();
            return Self::create_new_cache_folder(game_file_loader, texture_loader, map_loader, model_loader, game_file_hash);
        };

        let Ok(_hash) = Hash::from_hex(hash_file) else {
            #[cfg(feature = "debug")]
            print_debug!("Can't read game hash file. Deleting folder and create new cache");
            std::fs::remove_dir_all(folder_path).ok();
            return Self::create_new_cache_folder(game_file_loader, texture_loader, map_loader, model_loader, game_file_hash);
        };

        #[cfg(feature = "debug")]
        if _hash != game_file_hash {
            print_debug!("Cache is out of sync. Cache should be re-created");
        }

        Box::new(folder_archive)
    }

    fn create_new_cache_folder(
        game_file_loader: &GameFileLoader,
        texture_loader: Arc<TextureLoader>,
        map_loader: &MapLoader,
        model_loader: &ModelLoader,
        game_file_hash: Hash,
    ) -> Box<FolderArchive> {
        let mut map_files = game_file_loader.get_files_with_extension(MAP_FILE_EXTENSION);
        map_files.sort();

        #[cfg(feature = "debug")]
        let map_count = map_files.len();

        let mut archive = Box::new(FolderArchive::from_path(Path::new(CACHE_PATH_NAME)));

        for (_index, map_file) in map_files.iter().enumerate() {
            let path = Path::new(&map_file);
            let map_name = path.file_stem().unwrap().to_string_lossy().to_string();

            let mut textures = HashSet::new();

            match map_loader.collect_map_textures(model_loader, &mut textures, &map_name) {
                Ok(_) => {
                    let mut textures: Vec<String> = textures.into_iter().collect();
                    textures.sort();

                    let mut texture_atlas = UncompressedTextureAtlas::new(texture_loader.clone(), map_name.to_string(), true, true);
                    textures.iter().for_each(|texture| {
                        let _ = texture_atlas.register(texture);
                    });

                    texture_atlas.build_atlas();

                    #[cfg(feature = "debug")]
                    print_debug!(
                        "Creating texture atlas for map `{}`. {} of {} maps",
                        &map_name,
                        _index + 1,
                        map_count
                    );

                    let data = texture_atlas
                        .to_cached_texture_atlas()
                        .to_bytes()
                        .expect("can't convert cached texture atlas data to bytes");

                    let atlas_file_path = Self::get_texture_atlas_cache_base_path(&map_name, true, true);

                    archive.add_asset(&atlas_file_path, data, true);
                }
                Err(_err) => {
                    #[cfg(feature = "debug")]
                    print_debug!("Error while creating texture atlas for map `{}`: {:?}", map_name, _err);
                }
            };
        }

        let hash_string = format!("{:x?}", game_file_hash.as_bytes());
        archive.add_asset(HASH_FILE_PATH, hash_string.as_bytes().to_vec(), false);

        archive
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
        let data = self.archive.get_file_by_path(&data_path)?;

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
