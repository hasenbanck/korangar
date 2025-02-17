use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use blake3::Hash;
use derive_new::new;
use hashbrown::HashMap;
use image::codecs::tga::TgaEncoder;
use image::RgbaImage;
use korangar_util::container::{SecondarySimpleSlab, SimpleKey};
use korangar_util::texture_atlas::{AllocationId, AtlasAllocation};
use ragnarok_bytes::{ByteReader, ConversionResult, ConversionResultExt, FromBytes, ToBytes};

use crate::loaders::texture::TextureAtlasEntry;
use crate::loaders::TextureLoader;

// TODO: NHA use <Archive> instead
pub struct Cache {
    texture_loader: Arc<TextureLoader>,
    game_file_hash: Hash,
}

impl Cache {
    pub fn new(texture_loader: Arc<TextureLoader>, game_file_hash: Hash) -> Self {
        // TODO: NHA Read hash of cache. If it's not set then delete cache folder and
        //       create the cached files.

        // TODO: NHA If it's the wrong hash, then check
        //       every cached file and re-create, if source files have changed.

        Self {
            texture_loader,
            game_file_hash,
        }
    }

    fn get_texture_atlas_cache_base_path(name: &str, add_padding: bool, create_mip_map: bool) -> String {
        format!(
            "cache/atlas/{}_{}_{}.dat",
            name,
            if add_padding { "padded" } else { "unpadded" },
            if create_mip_map { "mip" } else { "nomip" }
        )
    }

    pub fn save_texture_atlas(
        &self,
        name: &str,
        add_padding: bool,
        create_mip_map: bool,
        cached_texture_atlas: CachedTextureAtlas,
        cached_texture_atlas_image: CachedTextureAtlasImage,
    ) {
        let data_path = Self::get_texture_atlas_cache_base_path(name, add_padding, create_mip_map);

        if let Some(parent) = Path::new(&data_path).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = File::create(&data_path).expect("can't create texture atlas cache file");

        let data = cached_texture_atlas
            .to_bytes()
            .expect("can't convert cached texture atlas data to bytes");
        file.write_all(&data).expect("can't write cache data to cache file");

        let data = cached_texture_atlas_image
            .to_bytes()
            .expect("can't convert cached texture atlas image to bytes");
        file.write_all(&data).expect("can't write cache data to cache file");
    }

    pub fn load_texture_atlas(
        &self,
        name: &str,
        add_padding: bool,
        create_mip_map: bool,
    ) -> Option<(CachedTextureAtlas, CachedTextureAtlasImage)> {
        let data_path = Self::get_texture_atlas_cache_base_path(name, add_padding, create_mip_map);
        let data = fs::read(&data_path).ok()?;
        let mut byte_reader = ByteReader::<()>::without_metadata(&data);
        let cached_atlas = CachedTextureAtlas::from_bytes(&mut byte_reader).ok()?;
        let cached_atlas_image = CachedTextureAtlasImage::from_bytes(&mut byte_reader).ok()?;
        Some((cached_atlas, cached_atlas_image))
    }
}

pub struct CachedTextureAtlas {
    pub lookup: HashMap<String, TextureAtlasEntry>,
    pub allocations: SecondarySimpleSlab<AllocationId, AtlasAllocation>,
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

        Ok(CachedTextureAtlas { lookup, allocations })
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
        atlas_data.to_bytes().trace::<Self>()
    }
}

#[derive(ToBytes, FromBytes)]
pub struct CachedTextureAtlasImage {
    pub width: u32,
    pub height: u32,
    pub mipmaps_count: u32,
    /// TGA encoded uncompressed image data
    pub uncompressed_data: Vec<u8>,
    /// BC7 encoded compressed image data
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
