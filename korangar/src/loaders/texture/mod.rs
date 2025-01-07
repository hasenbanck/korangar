use std::fs;
use std::io::Cursor;
use std::num::{NonZeroU32, NonZeroUsize};
use std::path::Path;
use std::sync::{Arc, Mutex};

use hashbrown::HashMap;
use image::{GrayImage, ImageBuffer, ImageFormat, ImageReader, Rgba, RgbaImage};
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_util::color::contains_transparent_pixel;
use korangar_util::container::{SecondarySimpleSlab, SimpleCache, SimpleKey};
use korangar_util::texture_atlas::{AllocationId, AtlasAllocation, OfflineTextureAtlas};
use korangar_util::FileLoader;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use wgpu::{
    CommandEncoderDescriptor, Device, Extent3d, Queue, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};

use super::error::LoadError;
use super::{FALLBACK_BMP_FILE, FALLBACK_JPEG_FILE, FALLBACK_PNG_FILE, FALLBACK_TGA_FILE, MIP_LEVELS};
use crate::graphics::{Lanczos3Drawer, MipMapRenderPassContext, Texture};
use crate::loaders::GameFileLoader;

const MAX_CACHE_COUNT: u32 = 512;
const MAX_CACHE_SIZE: usize = 512 * 1024 * 1024;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ImageType {
    Color,
    Sdf,
    Msdf,
}

pub struct TextureLoader {
    device: Arc<Device>,
    queue: Arc<Queue>,
    game_file_loader: Arc<GameFileLoader>,
    mip_map_render_context: MipMapRenderPassContext,
    lanczos3_drawer: Lanczos3Drawer,
    cache: Mutex<SimpleCache<(String, ImageType), Arc<Texture>>>,
}

impl TextureLoader {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, game_file_loader: Arc<GameFileLoader>) -> Self {
        let lanczos3_drawer = Lanczos3Drawer::new(&device);

        Self {
            device,
            queue,
            game_file_loader,
            mip_map_render_context: MipMapRenderPassContext::default(),
            lanczos3_drawer,
            cache: Mutex::new(SimpleCache::new(
                NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
                NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
            )),
        }
    }

    fn create(&self, name: &str, image: RgbaImage, transparent: bool) -> Arc<Texture> {
        let texture = Texture::new_with_data(
            &self.device,
            &self.queue,
            &TextureDescriptor {
                label: Some(name),
                size: Extent3d {
                    width: image.width(),
                    height: image.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            image.as_raw(),
            transparent,
        );
        Arc::new(texture)
    }

    pub fn create_sdf(&self, name: &str, image: GrayImage) -> Arc<Texture> {
        let texture = Texture::new_with_data(
            &self.device,
            &self.queue,
            &TextureDescriptor {
                label: Some(name),
                size: Extent3d {
                    width: image.width(),
                    height: image.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R8Unorm,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            image.as_raw(),
            false,
        );
        Arc::new(texture)
    }

    pub fn create_msdf(&self, name: &str, image: RgbaImage) -> Arc<Texture> {
        let texture = Texture::new_with_data(
            &self.device,
            &self.queue,
            &TextureDescriptor {
                label: Some(name),
                size: Extent3d {
                    width: image.width(),
                    height: image.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            image.as_raw(),
            false,
        );
        Arc::new(texture)
    }

    fn create_with_mip_maps(&self, name: &str, image: RgbaImage, transparent: bool) -> Arc<Texture> {
        let texture = Texture::new_with_data(
            &self.device,
            &self.queue,
            &TextureDescriptor {
                label: Some(name),
                size: Extent3d {
                    width: image.width(),
                    height: image.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: MIP_LEVELS,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            image.as_raw(),
            transparent,
        );

        let mut mip_views = Vec::with_capacity(MIP_LEVELS as usize);

        for level in 0..MIP_LEVELS {
            let view = texture.get_texture().create_view(&TextureViewDescriptor {
                label: Some(&format!("mip map level {level}")),
                format: None,
                dimension: Some(TextureViewDimension::D2),
                aspect: TextureAspect::All,
                base_mip_level: level,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: Some(1),
            });
            mip_views.push(view);
        }

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("TextureLoader"),
        });

        for index in 0..(MIP_LEVELS - 1) as usize {
            let mut pass = self
                .mip_map_render_context
                .create_pass(&self.device, &mut encoder, &mip_views[index], &mip_views[index + 1]);
            self.lanczos3_drawer.draw(&mut pass);
        }

        self.queue.submit(Some(encoder.finish()));

        Arc::new(texture)
    }

    pub fn load(&self, path: &str, image_type: ImageType) -> Result<Arc<Texture>, LoadError> {
        let texture = match image_type {
            ImageType::Color => {
                let (texture_data, transparent) = self.load_texture_data(path, false)?;
                self.create(path, texture_data, transparent)
            }
            ImageType::Sdf => {
                let texture_data = self.load_grayscale_texture_data(path)?;
                self.create_sdf(path, texture_data)
            }
            ImageType::Msdf => {
                let (texture_data, _) = self.load_texture_data(path, true)?;
                self.create_msdf(path, texture_data)
            }
        };

        self.cache
            .lock()
            .as_mut()
            .unwrap()
            .insert((path.to_string(), image_type), texture.clone())
            .unwrap();

        Ok(texture)
    }

    pub fn load_texture_data(&self, path: &str, raw: bool) -> Result<(RgbaImage, bool), LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load texture data from {}", path.magenta()));

        let image_format = match &path[path.len() - 4..] {
            ".bmp" | ".BMP" => ImageFormat::Bmp,
            ".jpg" | ".JPG" => ImageFormat::Jpeg,
            ".png" | ".PNG" => ImageFormat::Png,
            ".tga" | ".TGA" => ImageFormat::Tga,
            _ => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("File with unknown image format found: {:?}", path);
                    print_debug!("Replacing with fallback");
                }

                return self.load_texture_data(FALLBACK_PNG_FILE, raw);
            }
        };

        let file_data = match self.game_file_loader.get(&format!("data\\texture\\{path}")) {
            Ok(file_data) => file_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load image: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.load_texture_data(FALLBACK_PNG_FILE, raw);
            }
        };
        let reader = ImageReader::with_format(Cursor::new(file_data), image_format);

        let mut image_buffer = match reader.decode() {
            Ok(image) => image.to_rgba8(),
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to decode image: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                let fallback_path = match image_format {
                    ImageFormat::Bmp => FALLBACK_BMP_FILE,
                    ImageFormat::Jpeg => FALLBACK_JPEG_FILE,
                    ImageFormat::Png => FALLBACK_PNG_FILE,
                    ImageFormat::Tga => FALLBACK_TGA_FILE,
                    _ => unreachable!(),
                };

                return self.load_texture_data(fallback_path, raw);
            }
        };

        match image_format {
            ImageFormat::Bmp if !raw => {
                // These numbers are taken from https://github.com/Duckwhale/RagnarokFileFormats
                image_buffer
                    .pixels_mut()
                    .filter(|pixel| pixel.0[0] > 0xF0 && pixel.0[1] < 0x10 && pixel.0[2] > 0x0F)
                    .for_each(|pixel| *pixel = Rgba([0; 4]));
            }
            ImageFormat::Png | ImageFormat::Tga if !raw => {
                image_buffer = premultiply_alpha(image_buffer);
            }
            _ => {}
        }

        let transparent = match image_format == ImageFormat::Tga {
            true => contains_transparent_pixel(image_buffer.as_raw()),
            false => false,
        };

        #[cfg(feature = "debug")]
        timer.stop();

        Ok((image_buffer, transparent))
    }

    pub fn load_grayscale_texture_data(&self, path: &str) -> Result<GrayImage, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load grayscale texture data from {}", path.magenta()));

        let image_format = match &path[path.len() - 4..] {
            ".png" | ".PNG" => ImageFormat::Png,
            ".tga" | ".TGA" => ImageFormat::Tga,
            _ => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("File with unknown image format found: {:?}", path);
                    print_debug!("Replacing with fallback");
                }

                return self.load_grayscale_texture_data(FALLBACK_PNG_FILE);
            }
        };

        let file_data = match self.game_file_loader.get(&format!("data\\texture\\{path}")) {
            Ok(file_data) => file_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load image: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.load_grayscale_texture_data(FALLBACK_PNG_FILE);
            }
        };
        let reader = ImageReader::with_format(Cursor::new(file_data), image_format);

        let image_buffer = match reader.decode() {
            Ok(image) => image.to_luma8(),
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to decode image: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                let fallback_path = match image_format {
                    ImageFormat::Png => FALLBACK_PNG_FILE,
                    ImageFormat::Tga => FALLBACK_TGA_FILE,
                    _ => unreachable!(),
                };

                return self.load_grayscale_texture_data(fallback_path);
            }
        };

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(image_buffer)
    }

    pub fn get(&self, path: &str, image_type: ImageType) -> Option<Arc<Texture>> {
        let mut lock = self.cache.lock().unwrap();
        lock.get(&(path.into(), image_type)).cloned()
    }

    pub fn get_or_load(&self, path: &str, image_type: ImageType) -> Result<Arc<Texture>, LoadError> {
        let mut lock = self.cache.lock().unwrap();
        match lock.get(&(path.into(), image_type)) {
            Some(texture) => Ok(texture.clone()),
            None => {
                // We need to drop to avoid a deadlock here.
                drop(lock);
                self.load(path, image_type)
            }
        }
    }
}

fn premultiply_alpha(image_buffer: RgbaImage) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    // Iterating over "pixels_mut()" is considerably slower than iterating over the
    // raw bates, so we have to do this conversion to get raw, mutable access.
    let width = image_buffer.width();
    let height = image_buffer.height();
    let mut bytes = image_buffer.into_raw();

    korangar_util::color::premultiply_alpha(&mut bytes);

    RgbaImage::from_raw(width, height, bytes).unwrap()
}

pub struct TextureAtlasFactory {
    game_file_crc32: u32,
    name: String,
    texture_loader: Arc<TextureLoader>,
    texture_atlas: Atlas,
    lookup: HashMap<String, TextureAtlasEntry>,
    add_padding: bool,
    create_mip_map: bool,
    transparent: bool,
}

enum Atlas {
    Offline(OfflineTextureAtlas),
    Cache {
        image: RgbaImage,
        allocations: SecondarySimpleSlab<AllocationId, AtlasAllocation>,
    },
}

#[derive(Copy, Clone)]
pub struct TextureAtlasEntry {
    pub allocation_id: AllocationId,
    pub transparent: bool,
}

impl<'de> Deserialize<'de> for TextureAtlasEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            allocation_id: u32,
            transparent: bool,
        }

        let helper = Helper::deserialize(deserializer)?;

        Ok(Self {
            allocation_id: AllocationId::new(helper.allocation_id),
            transparent: helper.transparent,
        })
    }
}

impl Serialize for TextureAtlasEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("TextureAtlasEntry", 2)?;
        state.serialize_field("allocation_id", &self.allocation_id.key())?;
        state.serialize_field("transparent", &self.transparent)?;
        state.end()
    }
}

impl TextureAtlasFactory {
    #[cfg(feature = "debug")]
    pub fn create_from_group(
        game_file_crc32: u32,
        texture_loader: Arc<TextureLoader>,
        name: impl Into<String>,
        add_padding: bool,
        paths: &[&str],
    ) -> (Vec<AtlasAllocation>, Arc<Texture>) {
        let mut factory = Self::new_with_cache(game_file_crc32, texture_loader, name, add_padding, false);

        let mut ids: Vec<TextureAtlasEntry> = paths.iter().map(|path| factory.register(path)).collect();
        factory.build_atlas();

        let mapping = ids
            .drain(..)
            .map(|entry| factory.get_allocation(entry.allocation_id).unwrap())
            .collect();
        let texture = factory.upload_texture_atlas_texture();

        (mapping, texture)
    }

    fn new(
        game_file_crc32: u32,
        texture_loader: Arc<TextureLoader>,
        name: impl Into<String>,
        add_padding: bool,
        create_mip_map: bool,
    ) -> Self {
        let mip_level_count = if create_mip_map { NonZeroU32::new(MIP_LEVELS) } else { None };

        Self {
            game_file_crc32,
            name: name.into(),
            texture_loader,
            texture_atlas: Atlas::Offline(OfflineTextureAtlas::new(add_padding, mip_level_count)),
            lookup: HashMap::default(),
            add_padding,
            create_mip_map,
            transparent: false,
        }
    }

    pub fn new_with_cache(
        game_file_crc32: u32,
        texture_loader: Arc<TextureLoader>,
        name: impl Into<String>,
        add_padding: bool,
        create_mip_map: bool,
    ) -> Self {
        let name = name.into();

        dbg!(&name);

        if let Some((cache, image)) = Self::try_load_from_cache(game_file_crc32, &name, add_padding, create_mip_map) {
            dbg!("cached");
            Self {
                game_file_crc32,
                name,
                texture_loader,
                texture_atlas: Atlas::Cache {
                    image,
                    allocations: cache.allocations,
                },
                lookup: cache.lookup,
                add_padding,
                create_mip_map,
                transparent: false,
            }
        } else {
            dbg!("not cached");
            Self::new(game_file_crc32, texture_loader, name, add_padding, create_mip_map)
        }
    }

    /// Registers the given texture by its path. Will return an allocation ID
    /// which can later be used to get the actual allocation and flag that shows
    /// if a texture contains transparent pixels.
    pub fn register(&mut self, path: &str) -> TextureAtlasEntry {
        match &mut self.texture_atlas {
            Atlas::Offline(offline) => {
                if let Some(cached_entry) = self.lookup.get(path).copied() {
                    return cached_entry;
                }

                let (data, transparent) = self.texture_loader.load_texture_data(path, false).expect("can't load texture data");
                self.transparent |= transparent;
                let allocation_id = offline.register_image(data);

                let entry = TextureAtlasEntry {
                    allocation_id,
                    transparent,
                };
                self.lookup.insert(path.to_string(), entry);

                entry
            }
            Atlas::Cache { .. } => self.lookup.get(path).copied().unwrap(),
        }
    }

    pub fn get_allocation(&self, allocation_id: AllocationId) -> Option<AtlasAllocation> {
        match &self.texture_atlas {
            Atlas::Offline(atlas) => atlas.get_allocation(allocation_id),
            Atlas::Cache { allocations, .. } => allocations.get(allocation_id).copied(),
        }
    }

    pub fn build_atlas(&mut self) {
        if let Atlas::Offline(atlas) = &mut self.texture_atlas {
            atlas.build_atlas();
        }
    }

    fn get_cache_base_path(name: &str, add_padding: bool, create_mip_map: bool) -> String {
        format!(
            "cache/atlas_{}_{}_{}",
            name,
            if add_padding { "padded" } else { "unpadded" },
            if create_mip_map { "mip" } else { "nomip" }
        )
    }

    fn try_load_from_cache(
        game_file_crc32: u32,
        name: &str,
        add_padding: bool,
        create_mip_map: bool,
    ) -> Option<(TextureAtlasCache, RgbaImage)> {
        let base_path = Self::get_cache_base_path(name, add_padding, create_mip_map);
        let meta_path = format!("{}.ron", base_path);
        let image_path = format!("{}.tga", base_path);

        let cache_content = fs::read_to_string(&meta_path).ok()?;
        let cache: TextureAtlasCache = ron::from_str(&cache_content).ok()?;

        if cache.game_file_crc32 != game_file_crc32 {
            return None;
        }

        let image = image::open(&image_path).ok()?.into_rgba8();

        Some((cache, image))
    }

    fn save_to_cache(&self) {
        if let Atlas::Offline(atlas) = &self.texture_atlas {
            let base_path = Self::get_cache_base_path(&self.name, self.add_padding, self.create_mip_map);
            let meta_path = format!("{}.ron", base_path);
            let image_path = format!("{}.tga", base_path);

            if let Some(parent) = Path::new(&meta_path).parent() {
                fs::create_dir_all(parent).unwrap();
            }

            let cache = TextureAtlasCache {
                game_file_crc32: self.game_file_crc32,
                lookup: self.lookup.clone(),
                allocations: atlas.get_allocations(),
            };
            let serialized = ron::to_string(&cache).ok().unwrap();
            fs::write(meta_path, serialized).unwrap();

            atlas.save_atlas(&image_path).unwrap();
        }
    }

    pub fn upload_texture_atlas_texture(self) -> Arc<Texture> {
        self.save_to_cache();

        let image = match self.texture_atlas {
            Atlas::Offline(offline) => offline.get_atlas(),
            Atlas::Cache { image, .. } => image,
        };

        if self.create_mip_map {
            self.texture_loader
                .create_with_mip_maps(&format!("{} texture atlas", self.name), image, self.transparent)
        } else {
            self.texture_loader
                .create(&format!("{} texture atlas", self.name), image, self.transparent)
        }
    }
}

#[derive(Serialize, Deserialize)]
struct TextureAtlasCache {
    game_file_crc32: u32,
    lookup: HashMap<String, TextureAtlasEntry>,
    allocations: SecondarySimpleSlab<AllocationId, AtlasAllocation>,
}
