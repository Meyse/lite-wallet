//! Crop pixels cross IPC as bounded lossless PNG. Only the freshly encoded,
//! validated lossy WebP candidates can enter a publication request.
use super::codec::{validate_avatar_bytes, validate_header_bytes, AVATAR_MIME, MAX_AVATAR_BYTES};
use crate::types::WalletError;
use base64::{engine::general_purpose::STANDARD, Engine};
use image::GenericImageView;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedProfileImage {
    base64: String,
    mime_type: String,
    byte_length: usize,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileImageCandidates {
    balanced: EncodedProfileImage,
    smaller: Option<EncodedProfileImage>,
}

pub(crate) fn encode(kind: &str, png_base64: &str) -> Result<ProfileImageCandidates, WalletError> {
    let dimensions = match kind {
        "avatar" => (256, 256),
        "header" => (960, 160),
        _ => return Err(WalletError::IdentityProfileInvalidAvatar),
    };
    let invalid = || {
        if kind == "header" {
            WalletError::IdentityProfileInvalidHeader
        } else {
            WalletError::IdentityProfileInvalidAvatar
        }
    };
    if png_base64.len() > 1_048_576 {
        return Err(invalid());
    }
    let bytes = STANDARD.decode(png_base64).map_err(|_| invalid())?;
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(dimensions.0);
    limits.max_image_height = Some(dimensions.1);
    limits.max_alloc = Some(8 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| invalid())?;
    if image.dimensions() != dimensions {
        return Err(invalid());
    }
    let rgba = image.to_rgba8();
    let mut rgb = Vec::with_capacity(dimensions.0 as usize * dimensions.1 as usize * 3);
    for pixel in rgba.pixels() {
        let alpha = u32::from(pixel[3]);
        for value in &pixel.0[..3] {
            rgb.push(((u32::from(*value) * alpha + 255 * (255 - alpha) + 127) / 255) as u8);
        }
    }
    let encoder = webp::Encoder::from_rgb(&rgb, dimensions.0, dimensions.1);
    let mut balanced: Option<Vec<u8>> = None;
    let mut smaller: Option<Vec<u8>> = None;
    for quality in [86.0, 78.0, 70.0, 62.0, 55.0, 45.0] {
        let candidate = encoder
            .encode_simple(false, quality)
            .map_err(|_| invalid())?
            .to_vec();
        if candidate.len() > MAX_AVATAR_BYTES {
            continue;
        }
        if kind == "header" {
            validate_header_bytes(&candidate, AVATAR_MIME)?;
        } else {
            validate_avatar_bytes(&candidate, AVATAR_MIME)?;
        }
        if balanced
            .as_ref()
            .is_none_or(|value| value.len() > 16 * 1024)
        {
            balanced = Some(candidate.clone());
        }
        let single_target = candidate.len() <= 5 * 1024;
        if smaller
            .as_ref()
            .is_none_or(|value| candidate.len() < value.len())
        {
            smaller = Some(candidate);
        }
        if single_target {
            break;
        }
    }
    let balanced = balanced.ok_or_else(invalid)?;
    let smaller = smaller.filter(|value| value.len() < balanced.len());
    let media = |bytes: Vec<u8>| EncodedProfileImage {
        byte_length: bytes.len(),
        base64: STANDARD.encode(bytes),
        mime_type: AVATAR_MIME.into(),
    };
    Ok(ProfileImageCandidates {
        balanced: media(balanced),
        smaller: smaller.map(media),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn png(width: u32, height: u32) -> String {
        let image = image::RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 0])
        });
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        STANDARD.encode(out.into_inner())
    }
    #[test]
    fn native_encoder_produces_lossy_webp_with_correct_dimensions_and_white_matte() {
        for (kind, width, height) in [("avatar", 256, 256), ("header", 960, 160)] {
            let candidates = encode(kind, &png(width, height)).unwrap();
            assert_eq!(candidates.balanced.mime_type, AVATAR_MIME);
            let bytes = STANDARD.decode(candidates.balanced.base64).unwrap();
            assert_eq!(&bytes[12..16], b"VP8 "); // lossy, never the image crate's VP8L encoder
            let decoded =
                image::load_from_memory_with_format(&bytes, image::ImageFormat::WebP).unwrap();
            assert_eq!(decoded.dimensions(), (width, height));
            assert!(decoded
                .to_rgb8()
                .pixels()
                .all(|pixel| pixel.0.iter().all(|channel| *channel >= 250)));
            assert!(candidates.balanced.byte_length <= 32 * 1024);
        }
    }
    #[test]
    fn rejects_invalid_input_and_decode_dimensions_before_encoding() {
        assert!(encode("avatar", "invalid!").is_err());
        assert!(encode("other", &png(256, 256)).is_err());
        assert!(encode("header", &png(256, 256)).is_err());
        assert!(encode("avatar", &png(257, 256)).is_err());
        assert!(encode("avatar", &"A".repeat(1_048_577)).is_err());
    }
}
