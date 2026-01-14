use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use image::{DynamicImage, GenericImageView};

pub struct Texture {
    #[allow(unused)]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }

    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> anyhow::Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn from_bytes_array(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes_list: &Vec<Vec<u8>>,
        label: &str,
    ) -> Option<Self> {
        let imgs: Vec<DynamicImage> = bytes_list
            .iter()
            .map(|bytes| image::load_from_memory(bytes).unwrap())
            .collect();

        if imgs.is_empty() {
            return None;
        }

        Some(Self::from_image_list(device, queue, &imgs, Some(label)).unwrap())
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> anyhow::Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    pub fn from_image_list(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        imgs: &Vec<image::DynamicImage>,
        label: Option<&str>,
    ) -> anyhow::Result<Self> {
        let size = get_img_size_if_all_equal(&imgs)?;
        let layers = size.depth_or_array_layers;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        for (i, img) in imgs.iter().enumerate() {
            let rgba = img.to_rgba8();
            let raw = rgba.as_raw();

            let bytes_per_row = 4 * size.width;
            let rows_per_image = size.height;

            queue.write_texture(
                wgpu::TexelCopyTextureInfoBase {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                raw,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(rows_per_image),
                },
                wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            array_layer_count: Some(layers),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }
}

fn get_img_size_if_all_equal(imgs: &Vec<image::DynamicImage>) -> anyhow::Result<wgpu::Extent3d> {
    if imgs.is_empty() {
        anyhow::bail!("Empty image list");
    }

    let (w, h) = imgs[0].dimensions();

    for img in imgs.iter().skip(1) {
        let (ww, hh) = img.dimensions();
        anyhow::ensure!(ww == w && hh == h, "image dimensions must match");
    }

    Ok(wgpu::Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: imgs.len() as u32,
    })
}

pub fn load_asset(rel_path: &str) -> anyhow::Result<Vec<u8>> {
    // Reject absolute paths to enforce assets rooted under `res/` by default.
    let rel_path = Path::new(rel_path);
    if rel_path.is_absolute() {
        anyhow::bail!(
            "expected relative asset path, got absolute: {}",
            rel_path.display()
        );
    }

    // Candidate roots in order (ASSETS_DIR from build.rs, then project res, exe-res, cwd/res)
    let candidates: Vec<PathBuf> = vec![
        option_env!("ASSETS_DIR").map(PathBuf::from),
        Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("res")),
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.join("res"))),
        std::env::current_dir().ok().map(|cwd| cwd.join("res")),
    ]
    .into_iter()
    .flatten()
    .collect();

    for root in candidates {
        let full = root.join(rel_path);

        // Skip if file doesn't exist at this root
        if !full.exists() {
            continue;
        }

        // canonicalize both root and file to protect against path traversal (..)
        let canon_root = root
            .canonicalize()
            .with_context(|| format!("failed to canonicalize assets root {}", root.display()))?;
        let canon_full = full
            .canonicalize()
            .with_context(|| format!("failed to canonicalize asset path {}", full.display()))?;

        // Ensure the final path is inside the assets root
        if !canon_full.starts_with(&canon_root) {
            // This means rel_path tried to escape the assets dir.
            continue;
        }

        // Finally read and return the bytes
        let bytes = fs::read(&canon_full)
            .with_context(|| format!("failed to read asset {}", canon_full.display()))?;
        return Ok(bytes);
    }

    anyhow::bail!("asset not found: {}", rel_path.display());
}
