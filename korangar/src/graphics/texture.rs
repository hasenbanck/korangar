use std::fmt::{Debug, Formatter};
use std::num::NonZeroU32;
use std::sync::{Arc, OnceLock};

use derive_new::new;
use wgpu::util::{DeviceExt, TextureDataOrder};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, Device, Extent3d, Features, Queue, ShaderStages, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
};

use crate::graphics::features_supported;
use crate::interface::layout::ScreenSize;
use crate::MAX_BINDING_TEXTURE_ARRAY_COUNT;

pub struct Texture {
    label: Option<String>,
    texture: wgpu::Texture,
    texture_view: TextureView,
}

impl Debug for Texture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.label {
            None => write!(f, "Texture(\"Unknown\")"),
            Some(label) => write!(f, "Texture(\"{label}\")"),
        }
    }
}

impl Texture {
    pub fn new(device: &Device, descriptor: &TextureDescriptor) -> Self {
        let label = descriptor.label.map(|label| label.to_string());
        let texture = device.create_texture(descriptor);
        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: descriptor.label,
            ..Default::default()
        });

        Self {
            label,
            texture,
            texture_view,
        }
    }

    pub fn new_with_data(device: &Device, queue: &Queue, descriptor: &TextureDescriptor, data: &[u8]) -> Self {
        let label = descriptor.label.map(|label| label.to_string());
        let texture = device.create_texture_with_data(queue, descriptor, TextureDataOrder::LayerMajor, data);
        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: descriptor.label,
            ..Default::default()
        });

        Self {
            label,
            texture,
            texture_view,
        }
    }

    pub fn get_extent(&self) -> Extent3d {
        self.texture.size()
    }

    pub fn get_format(&self) -> TextureFormat {
        self.texture.format()
    }

    pub fn get_texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn get_texture_view(&self) -> &TextureView {
        &self.texture_view
    }
}

pub struct TextureGroup {
    _textures: Vec<Arc<Texture>>,
    bind_group: BindGroup,
}

impl TextureGroup {
    pub fn bind_group_layout(device: &Device) -> &'static BindGroupLayout {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("texture group"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: NonZeroU32::new(MAX_BINDING_TEXTURE_ARRAY_COUNT as u32),
                }],
            })
        })
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    pub(crate) fn new(device: &Device, label: &str, textures: Vec<Arc<Texture>>) -> Self {
        let texture_count = textures.len();
        let mut texture_views: Vec<&TextureView> = textures
            .iter()
            .take(MAX_BINDING_TEXTURE_ARRAY_COUNT.min(texture_count))
            .map(|texture| texture.get_texture_view())
            .collect();

        if !features_supported(Features::PARTIALLY_BOUND_BINDING_ARRAY) {
            for _ in 0..MAX_BINDING_TEXTURE_ARRAY_COUNT.saturating_sub(texture_count) {
                texture_views.push(texture_views[0]);
            }
        }

        let layout = Self::bind_group_layout(device);
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureViewArray(&texture_views),
            }],
        });

        Self {
            _textures: textures,
            bind_group,
        }
    }
}

pub struct CubeTexture {
    label: Option<String>,
    texture: wgpu::Texture,
    texture_view: TextureView,
    texture_face_views: [TextureView; 6],
}

impl Debug for CubeTexture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.label {
            None => write!(f, "CubeTexture(\"Unknown\")"),
            Some(label) => write!(f, "CubeTexture(\"{label}\")"),
        }
    }
}

impl CubeTexture {
    pub fn new(device: &Device, descriptor: &TextureDescriptor) -> Self {
        let label = descriptor.label.map(|label| label.to_string());
        let texture = device.create_texture(descriptor);

        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: descriptor.label,
            format: None,
            dimension: Some(TextureViewDimension::Cube),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: Some(6),
        });

        fn create_face_view(texture: &wgpu::Texture, index: u32) -> TextureView {
            texture.create_view(&TextureViewDescriptor {
                label: Some("cube map face view"),
                format: None,
                dimension: Some(TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: index,
                array_layer_count: Some(1),
            })
        }

        let texture_face_views = [
            create_face_view(&texture, 0),
            create_face_view(&texture, 1),
            create_face_view(&texture, 2),
            create_face_view(&texture, 3),
            create_face_view(&texture, 4),
            create_face_view(&texture, 5),
        ];

        Self {
            label,
            texture,
            texture_view,
            texture_face_views,
        }
    }

    pub fn get_texture_format(&self) -> TextureFormat {
        self.texture.format()
    }

    pub fn get_texture_view(&self) -> &TextureView {
        &self.texture_view
    }

    pub fn get_texture_face_view(&self, index: usize) -> &TextureView {
        &self.texture_face_views[index]
    }
}

pub(crate) enum TextureType {
    ColorAttachment,
    DepthAttachment,
    Depth,
}

impl From<TextureType> for TextureUsages {
    fn from(value: TextureType) -> Self {
        match value {
            TextureType::ColorAttachment => TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            TextureType::DepthAttachment => TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            TextureType::Depth => TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        }
    }
}

#[derive(new)]
pub(crate) struct TextureFactory<'a> {
    device: &'a Device,
    dimensions: ScreenSize,
    sample_count: u32,
}

impl<'a> TextureFactory<'a> {
    pub(crate) fn new_texture(&self, texture_name: &str, format: TextureFormat, attachment_image_type: TextureType) -> Texture {
        Texture::new(self.device, &TextureDescriptor {
            label: Some(texture_name),
            size: Extent3d {
                width: self.dimensions.width as u32,
                height: self.dimensions.height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: self.sample_count,
            dimension: TextureDimension::D2,
            format,
            usage: attachment_image_type.into(),
            view_formats: &[],
        })
    }

    pub(crate) fn new_cube_texture(&self, texture_name: &str, format: TextureFormat, attachment_image_type: TextureType) -> CubeTexture {
        CubeTexture::new(self.device, &TextureDescriptor {
            label: Some(texture_name),
            size: Extent3d {
                width: self.dimensions.width as u32,
                height: self.dimensions.height as u32,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: attachment_image_type.into(),
            view_formats: &[],
        })
    }
}
