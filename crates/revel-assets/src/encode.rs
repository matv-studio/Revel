//! Décodage, redimensionnement et encodage.

use crate::{Format, Variant};
use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer};
use image::{DynamicImage, ImageDecoder, ImageReader, metadata::Orientation};
use std::path::Path;

pub(crate) type Err = Box<dyn std::error::Error + Send + Sync>;

/// Dimensions affichées (orientation EXIF comprise), sans décoder les pixels.
pub(crate) fn dimensions(path: &Path) -> Result<(u32, u32), Err> {
    let mut dec = ImageReader::open(path)?
        .with_guessed_format()?
        .into_decoder()?;
    let (w, h) = dec.dimensions();
    let o = dec.orientation().unwrap_or(Orientation::NoTransforms);
    Ok(match o {
        Orientation::Rotate90
        | Orientation::Rotate270
        | Orientation::Rotate90FlipH
        | Orientation::Rotate270FlipH => (h, w),
        _ => (w, h),
    })
}

/// Image décodée, orientation EXIF appliquée.
pub(crate) fn decode(path: &Path) -> Result<DynamicImage, Err> {
    let mut dec = ImageReader::open(path)?
        .with_guessed_format()?
        .into_decoder()?;
    let o = dec.orientation().unwrap_or(Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(dec)?;
    img.apply_orientation(o);
    Ok(img)
}

pub(crate) fn variant(img: &DynamicImage, v: &Variant) -> Result<Vec<u8>, Err> {
    let has_alpha = img.color().has_alpha();
    let src = if has_alpha {
        DynamicImage::ImageRgba8(img.to_rgba8())
    } else {
        DynamicImage::ImageRgb8(img.to_rgb8())
    };
    let resized = if src.width() == v.width && src.height() == v.height {
        src
    } else {
        let mut dst = DynamicImage::new(v.width, v.height, src.color());
        Resizer::new().resize(
            &src,
            &mut dst,
            &ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3)),
        )?;
        dst
    };
    let (w, h) = (resized.width(), resized.height());
    Ok(match v.format {
        Format::Avif => {
            let rgba = resized.to_rgba8();
            let px: Vec<rgb::RGBA8> = rgba
                .pixels()
                .map(|p| rgb::RGBA8::new(p[0], p[1], p[2], p[3]))
                .collect();
            let enc = ravif::Encoder::new()
                .with_quality(v.quality.avif as f32)
                .with_alpha_quality(v.quality.avif as f32)
                .with_speed(8);
            enc.encode_rgba(ravif::Img::new(&px[..], w as usize, h as usize))?
                .avif_file
        }
        Format::Webp => {
            let mem = if has_alpha {
                let rgba = resized.to_rgba8();
                webp::Encoder::from_rgba(&rgba, w, h).encode(v.quality.webp as f32)
            } else {
                let rgb = resized.to_rgb8();
                webp::Encoder::from_rgb(&rgb, w, h).encode(v.quality.webp as f32)
            };
            mem.to_vec()
        }
        Format::Jpeg => {
            let rgb = resized.to_rgb8();
            let mut out = Vec::new();
            let mut enc = jpeg_encoder::Encoder::new(&mut out, v.quality.jpeg);
            enc.set_progressive(true);
            enc.encode(&rgb, w as u16, h as u16, jpeg_encoder::ColorType::Rgb)?;
            out
        }
        Format::Png => {
            let mut out = Vec::new();
            resized.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)?;
            out
        }
        Format::Gif => {
            let mut out = Vec::new();
            resized.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Gif)?;
            out
        }
    })
}
