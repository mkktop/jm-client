use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ImageFormat, ImageEncoder, RgbImage};

use super::protocol::md5_hex;

/// JM 把章节图按纵向切块后打乱。这里计算块数。
///
/// `scramble_id` 来自 `/chapter_view_template`，`chapter_id` 是章节/图片 id，
/// `filename` 可以带后缀，内部会去掉后缀。
pub fn block_count(scramble_id: i64, chapter_id: i64, filename: &str) -> u32 {
    if chapter_id < scramble_id {
        return 0;
    }
    if chapter_id < 268_850 {
        return 10;
    }
    let modulo = if chapter_id < 421_926 { 10 } else { 8 };
    let stem = filename.rsplit_once('.').map(|(s, _)| s).unwrap_or(filename);
    let hash = md5_hex(&format!("{chapter_id}{stem}"));
    let last = hash.chars().last().unwrap_or('0') as u32;
    (last % modulo) * 2 + 2
}

/// 把纵向切块后的图片还原：块顺序上下反转，余数行归最底块。
pub fn stitch(src: &RgbImage, block_num: u32) -> RgbImage {
    if block_num <= 1 {
        return src.clone();
    }
    let (width, height) = src.dimensions();
    let mut dst = RgbImage::new(width, height);
    let remainder = height % block_num;
    for i in 0..block_num {
        let mut block_height = height / block_num;
        let src_y_start = height
            .saturating_sub(block_height * (i + 1))
            .saturating_sub(remainder);
        let mut dst_y_start = block_height * i;
        if i == 0 {
            block_height += remainder;
        } else {
            dst_y_start += remainder;
        }
        for y in 0..block_height {
            let src_y = src_y_start + y;
            let dst_y = dst_y_start + y;
            if src_y >= height || dst_y >= height {
                continue;
            }
            for x in 0..width {
                dst.put_pixel(x, dst_y, *src.get_pixel(x, src_y));
            }
        }
    }
    dst
}

pub fn encode(img: &RgbImage, format: &str) -> Result<Vec<u8>, String> {
    let mut buffer = Cursor::new(Vec::new());
    match format.to_ascii_lowercase().as_str() {
        "png" => {
            let encoder =
                PngEncoder::new_with_quality(&mut buffer, CompressionType::Best, FilterType::default());
            encoder
                .write_image(
                    img.as_raw(),
                    img.width(),
                    img.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| format!("PNG 编码失败: {e}"))?;
        }
        "webp" => {
            img.write_to(&mut buffer, ImageFormat::WebP)
                .map_err(|e| format!("WebP 编码失败: {e}"))?;
        }
        _ => {
            let mut encoder = JpegEncoder::new_with_quality(&mut buffer, 92);
            encoder
                .encode(
                    img.as_raw(),
                    img.width(),
                    img.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| format!("JPEG 编码失败: {e}"))?;
        }
    }
    Ok(buffer.into_inner())
}

pub fn extension_for(format: &str) -> &'static str {
    match format.to_ascii_lowercase().as_str() {
        "png" => "png",
        "webp" => "webp",
        _ => "jpg",
    }
}

fn is_gif(bytes: &[u8]) -> bool {
    image::guess_format(bytes)
        .map(|format| format == ImageFormat::Gif)
        .unwrap_or(false)
}

fn content_type(bytes: &[u8]) -> &'static str {
    match image::guess_format(bytes) {
        Ok(ImageFormat::Gif) => "image/gif",
        Ok(ImageFormat::Png) => "image/png",
        Ok(ImageFormat::Jpeg) => "image/jpeg",
        _ => "image/webp",
    }
}

/// 用于在线阅读：不需要还原时直接返回原始字节，避免 CPU 开销。
pub fn process_for_display(
    scramble_id: i64,
    chapter_id: i64,
    filename: &str,
    bytes: &[u8],
) -> Result<(Vec<u8>, &'static str), String> {
    if is_gif(bytes) {
        return Ok((bytes.to_vec(), "image/gif"));
    }
    let block_num = block_count(scramble_id, chapter_id, filename);
    if block_num == 0 {
        return Ok((bytes.to_vec(), content_type(bytes)));
    }
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("图片解码失败: {e}"))?
        .to_rgb8();
    let stitched = stitch(&img, block_num);
    // 在线阅读用 JPEG，避免无损 WebP 把单页放大到 9MB 级别。
    Ok((encode(&stitched, "jpg")?, "image/jpeg"))
}

/// 用于下载：非 GIF 一律解码、还原、按用户格式重新编码。
pub fn process_for_download(
    scramble_id: i64,
    chapter_id: i64,
    filename: &str,
    bytes: &[u8],
    output_format: &str,
) -> Result<(Vec<u8>, &'static str), String> {
    if is_gif(bytes) {
        return Ok((bytes.to_vec(), "gif"));
    }
    let block_num = block_count(scramble_id, chapter_id, filename);
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("图片解码失败: {e}"))?
        .to_rgb8();
    let processed = if block_num == 0 {
        img
    } else {
        stitch(&img, block_num)
    };
    Ok((
        encode(&processed, output_format)?,
        extension_for(output_format),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn no_scramble_below_threshold() {
        assert_eq!(block_count(220_980, 100, "00001.webp"), 0);
    }

    #[test]
    fn fixed_ten_blocks_in_old_range() {
        assert_eq!(block_count(220_980, 240_000, "00001.webp"), 10);
    }

    #[test]
    fn block_count_is_even_and_within_range() {
        let count = block_count(220_980, 1_485_105, "00001.webp");
        assert!((2..=16).contains(&count));
        assert_eq!(count % 2, 0);
    }

    #[test]
    fn stitch_reverses_vertical_blocks() {
        let mut src = RgbImage::new(1, 6);
        // rows: 0,1,2,3,4,5 -> values 10,11,12,13,14,15
        for y in 0..6 {
            src.put_pixel(0, y, Rgb([(10 + y) as u8, 0, 0]));
        }
        let out = stitch(&src, 3);
        // bottom block first, then middle, then top
        let values: Vec<u8> = (0..6).map(|y| out.get_pixel(0, y)[0]).collect();
        assert_eq!(values, vec![14, 15, 12, 13, 10, 11]);
    }
}
