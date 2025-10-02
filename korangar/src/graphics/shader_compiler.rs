use std::borrow::Cow;
use std::io::Cursor;

use hashbrown::HashMap;
use sevenz_rust2::{Archive, BlockDecoder, Password};
use wgpu::{Device, ShaderModule, ShaderModuleDescriptor, ShaderModuleDescriptorPassthrough, ShaderRuntimeChecks, ShaderSource};

static ARCHIVE_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders_compiled/shaders.7z"));

#[derive(Copy, Clone)]
struct FileEntry {
    file_crc: u64,
    file_size: u64,
    block_index: usize,
}

pub struct ShaderCompiler {
    device: Device,
    use_shader_passthrough: bool,
    files: HashMap<String, FileEntry>,
    archive: Archive,
    password: Password,
}

impl ShaderCompiler {
    pub fn new(device: Device, use_shader_passthrough: bool) -> Self {
        let password = Password::empty();
        let archive = Archive::read(&mut Cursor::new(ARCHIVE_DATA), &password).expect("failed to read archive");
        let mut files = HashMap::with_capacity(archive.files.len());
        for (entry, file_block_index) in
            archive
                .files
                .iter()
                .zip(archive.stream_map.file_block_index.iter())
                .filter_map(|(entry, file_block_index)| {
                    if !entry.is_directory
                        && !entry.is_anti_item
                        && let Some(file_block_index) = *file_block_index
                    {
                        Some((entry, file_block_index))
                    } else {
                        None
                    }
                })
        {
            files.insert(entry.name.clone(), FileEntry {
                file_crc: entry.crc,
                file_size: entry.size,
                block_index: file_block_index,
            });
        }

        Self {
            device,
            use_shader_passthrough,
            files,
            archive,
            password,
        }
    }

    pub fn create_shader_module(&self, folder: &str, name: &str) -> ShaderModule {
        let path = format!("{folder}/{name}.spv");
        let shader_name = format!("{folder}/{name}");

        let file_entry = *self
            .files
            .get(&path)
            .unwrap_or_else(|| panic!("failed to get shader module for {folder}/{name}"));

        let mut cursor = Cursor::new(ARCHIVE_DATA);

        let block_decoder = BlockDecoder::new(1, file_entry.block_index, &self.archive, &self.password, &mut cursor);

        let mut aligned_data = vec![0u32; (file_entry.file_size / 4) as usize];
        let copy_target = bytemuck::cast_slice_mut(&mut aligned_data);

        let mut found = false;

        block_decoder
            .for_each_entries(&mut |entry, reader| {
                if file_entry.file_crc == entry.crc && file_entry.file_size == entry.size {
                    let _ = reader.read_exact(copy_target);
                    found = true;
                }
                Ok(false)
            })
            .expect("could not decompress shader module");

        assert!(found, "failed to read shader data for {folder}/{name}");

        match self.use_shader_passthrough {
            true => unsafe {
                self.device.create_shader_module_passthrough(ShaderModuleDescriptorPassthrough {
                    entry_point: String::new(),
                    label: Some(&shader_name),
                    num_workgroups: (0, 0, 0),
                    runtime_checks: ShaderRuntimeChecks::unchecked(),
                    spirv: Some(Cow::Owned(aligned_data)),
                    dxil: None,
                    msl: None,
                    hlsl: None,
                    glsl: None,
                    wgsl: None,
                })
            },
            false => self.device.create_shader_module(ShaderModuleDescriptor {
                label: Some(&shader_name),
                source: ShaderSource::SpirV(Cow::Owned(aligned_data)),
            }),
        }
    }
}
