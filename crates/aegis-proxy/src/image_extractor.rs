//! Image extraction from API responses (F033).
//!
//! Extracts images from various AI image generation API response formats,
//! including base64-encoded JSON responses and binary image data.

use serde_json::Value;

/// Extracted image data from an API response.
#[derive(Debug, Clone)]
pub struct ExtractedImage {
    /// The raw image bytes.
    pub data: Vec<u8>,
    /// The image format (mime type), if known.
    pub format: Option<String>,
    /// Source location in the response (e.g., "artifacts[0].base64").
    pub source_path: String,
    /// Index in the response (for multiple images).
    pub index: usize,
}

impl ExtractedImage {
    /// Creates a new extracted image.
    pub fn new(data: Vec<u8>, source_path: impl Into<String>, index: usize) -> Self {
        Self {
            data,
            format: None,
            source_path: source_path.into(),
            index,
        }
    }

    /// Sets the image format.
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Returns true if this is likely a valid image based on magic bytes.
    pub fn is_valid_image(&self) -> bool {
        detect_image_format(&self.data).is_some()
    }

    /// Detects and sets the format from magic bytes.
    pub fn detect_format(&mut self) {
        if let Some(format) = detect_image_format(&self.data) {
            self.format = Some(format.to_string());
        }
    }
}

/// Detects image format from magic bytes.
pub fn detect_image_format(data: &[u8]) -> Option<&'static str> {
    if data.len() < 4 {
        return None;
    }

    // JPEG: FF D8 FF
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }

    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("image/png");
    }

    // GIF: GIF87a or GIF89a
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Some("image/gif");
    }

    // WebP: RIFF....WEBP
    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        return Some("image/webp");
    }

    // BMP: BM
    if data.starts_with(b"BM") {
        return Some("image/bmp");
    }

    None
}

/// Extracts images from a JSON API response body.
///
/// Supports various image generation API formats:
/// - OpenAI: `{"data": [{"b64_json": "..."}]}` or `{"data": [{"url": "..."}]}`
/// - Stability AI: `{"artifacts": [{"base64": "..."}]}`
/// - Leonardo.ai: `{"generations": [{"url": "..."}]}` or base64
/// - xAI Grok: `{"images": [{"image": "..."}]}`
/// - Replicate: `{"output": ["data:image/png;base64,..."]}`
/// - Together AI: `{"data": [{"b64_json": "..."}]}`
/// - Generic: Searches for common base64 image patterns
pub fn extract_images_from_json(body: &[u8]) -> Vec<ExtractedImage> {
    let Ok(json) = serde_json::from_slice::<Value>(body) else {
        return Vec::new();
    };

    let mut images = Vec::new();

    // Try OpenAI/Together AI format: data[].b64_json
    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
        for (i, item) in data.iter().enumerate() {
            if let Some(b64) = item.get("b64_json").and_then(|v| v.as_str()) {
                if let Some(img) = decode_base64_image(b64) {
                    images.push(ExtractedImage::new(img, format!("data[{}].b64_json", i), i));
                }
            }
            // Also check for data URI format
            if let Some(url) = item.get("url").and_then(|v| v.as_str()) {
                if url.starts_with("data:image/") {
                    if let Some(img) = decode_data_uri(url) {
                        images.push(ExtractedImage::new(img, format!("data[{}].url", i), i));
                    }
                }
            }
        }
    }

    // Try Stability AI format: artifacts[].base64
    if let Some(artifacts) = json.get("artifacts").and_then(|a| a.as_array()) {
        for (i, artifact) in artifacts.iter().enumerate() {
            if let Some(b64) = artifact.get("base64").and_then(|v| v.as_str()) {
                if let Some(img) = decode_base64_image(b64) {
                    images.push(ExtractedImage::new(
                        img,
                        format!("artifacts[{}].base64", i),
                        images.len(),
                    ));
                }
            }
        }
    }

    // Try xAI Grok format: images[].image
    if let Some(imgs) = json.get("images").and_then(|i| i.as_array()) {
        for (i, img_obj) in imgs.iter().enumerate() {
            if let Some(b64) = img_obj.get("image").and_then(|v| v.as_str()) {
                if let Some(img) = decode_base64_image(b64) {
                    images.push(ExtractedImage::new(
                        img,
                        format!("images[{}].image", i),
                        images.len(),
                    ));
                }
            }
        }
    }

    // Try Replicate format: output[] (array of data URIs or URLs)
    if let Some(output) = json.get("output").and_then(|o| o.as_array()) {
        for (i, item) in output.iter().enumerate() {
            if let Some(url) = item.as_str() {
                if url.starts_with("data:image/") {
                    if let Some(img) = decode_data_uri(url) {
                        images.push(ExtractedImage::new(
                            img,
                            format!("output[{}]", i),
                            images.len(),
                        ));
                    }
                }
            }
        }
    }

    // Try Leonardo.ai format: generations_by_pk.generated_images[].url (data URI)
    // or just generated_images[].url
    if let Some(generations) = json
        .get("generations_by_pk")
        .and_then(|g| g.get("generated_images"))
        .and_then(|gi| gi.as_array())
        .or_else(|| json.get("generated_images").and_then(|gi| gi.as_array()))
    {
        for (i, gen) in generations.iter().enumerate() {
            if let Some(url) = gen.get("url").and_then(|v| v.as_str()) {
                if url.starts_with("data:image/") {
                    if let Some(img) = decode_data_uri(url) {
                        images.push(ExtractedImage::new(
                            img,
                            format!("generated_images[{}].url", i),
                            images.len(),
                        ));
                    }
                }
            }
        }
    }

    // Try Ideogram format: data[].url or data[].image_url (may be data URIs)
    if images.is_empty() {
        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
            for (i, item) in data.iter().enumerate() {
                for key in &["image_url", "image", "b64"] {
                    if let Some(val) = item.get(*key).and_then(|v| v.as_str()) {
                        if val.starts_with("data:image/") {
                            if let Some(img) = decode_data_uri(val) {
                                images.push(ExtractedImage::new(
                                    img,
                                    format!("data[{}].{}", i, key),
                                    images.len(),
                                ));
                            }
                        } else if looks_like_base64(val) {
                            if let Some(img) = decode_base64_image(val) {
                                images.push(ExtractedImage::new(
                                    img,
                                    format!("data[{}].{}", i, key),
                                    images.len(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // Generic fallback: search for any base64 strings that look like images
    if images.is_empty() {
        images.extend(extract_generic_base64_images(&json));
    }

    // Detect formats for all images
    for img in &mut images {
        img.detect_format();
    }

    images
}

/// Extracts images from a binary response body.
///
/// If the content is a valid image (detected by magic bytes), returns it.
pub fn extract_image_from_binary(
    body: &[u8],
    content_type: Option<&str>,
) -> Option<ExtractedImage> {
    // Check if it's a valid image by magic bytes
    if detect_image_format(body).is_some() {
        let mut img = ExtractedImage::new(body.to_vec(), "binary_response", 0);

        // Use content-type if provided, otherwise detect
        if let Some(ct) = content_type {
            if ct.starts_with("image/") {
                img.format = Some(ct.to_string());
            }
        }
        img.detect_format();

        return Some(img);
    }

    None
}

/// Extracts images from multipart form-data request bodies (for upload filtering).
///
/// Returns images found in the multipart body along with their field names.
pub fn extract_images_from_multipart(body: &[u8], boundary: &str) -> Vec<(String, ExtractedImage)> {
    let mut images = Vec::new();
    let boundary_bytes = format!("--{}", boundary).into_bytes();

    // Simple multipart parsing - look for Content-Disposition and image data
    let mut current_pos = 0;
    let mut part_index = 0;

    while let Some(start) = find_subsequence(&body[current_pos..], &boundary_bytes) {
        let abs_start = current_pos + start + boundary_bytes.len();

        // Find end of this part (next boundary or end)
        let end = find_subsequence(&body[abs_start..], &boundary_bytes)
            .map(|e| abs_start + e)
            .unwrap_or(body.len());

        if abs_start < end {
            let part = &body[abs_start..end];

            // Parse headers and body
            if let Some((field_name, part_body)) = parse_multipart_part(part) {
                // Check if it's an image
                if let Some(format) = detect_image_format(part_body) {
                    let mut img = ExtractedImage::new(
                        part_body.to_vec(),
                        format!("multipart.{}", field_name),
                        part_index,
                    );
                    img.format = Some(format.to_string());
                    images.push((field_name, img));
                }
            }

            part_index += 1;
        }

        current_pos = end;
        if current_pos >= body.len() {
            break;
        }
    }

    images
}

/// Decodes a base64-encoded image string.
fn decode_base64_image(b64: &str) -> Option<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    // Handle data URI prefix if present
    let b64_str = if let Some(pos) = b64.find(",") {
        &b64[pos + 1..]
    } else {
        b64
    };

    // Clean up whitespace and newlines
    let cleaned: String = b64_str.chars().filter(|c| !c.is_whitespace()).collect();

    STANDARD.decode(&cleaned).ok()
}

/// Decodes a data URI (e.g., "data:image/png;base64,...")
fn decode_data_uri(uri: &str) -> Option<Vec<u8>> {
    if !uri.starts_with("data:") {
        return None;
    }

    // Find the comma separating metadata from data
    let comma_pos = uri.find(',')?;
    let data = &uri[comma_pos + 1..];

    // Check if it's base64 encoded
    let metadata = &uri[5..comma_pos];
    if metadata.contains("base64") {
        decode_base64_image(data)
    } else {
        // URL-encoded data (rare for images)
        None
    }
}

/// Checks if a string looks like base64-encoded data.
fn looks_like_base64(s: &str) -> bool {
    // Base64 strings are typically long and contain only valid characters
    s.len() > 100
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
}

/// Recursively searches JSON for base64 image strings.
fn extract_generic_base64_images(json: &Value) -> Vec<ExtractedImage> {
    let mut images = Vec::new();
    extract_base64_recursive(json, "", &mut images);
    images
}

fn extract_base64_recursive(value: &Value, path: &str, images: &mut Vec<ExtractedImage>) {
    match value {
        Value::String(s) => {
            // Check for data URI
            if s.starts_with("data:image/") {
                if let Some(img) = decode_data_uri(s) {
                    images.push(ExtractedImage::new(img, path, images.len()));
                }
            }
            // Check for raw base64 that might be an image
            else if looks_like_base64(s) {
                if let Some(img) = decode_base64_image(s) {
                    // Verify it's actually an image
                    if detect_image_format(&img).is_some() {
                        images.push(ExtractedImage::new(img, path, images.len()));
                    }
                }
            }
        }
        Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                let new_path = format!("{}[{}]", path, i);
                extract_base64_recursive(item, &new_path, images);
            }
        }
        Value::Object(obj) => {
            for (key, val) in obj {
                let new_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };
                extract_base64_recursive(val, &new_path, images);
            }
        }
        _ => {}
    }
}

/// Parses a multipart part to extract field name and body.
fn parse_multipart_part(part: &[u8]) -> Option<(String, &[u8])> {
    // Find header/body separator (double CRLF)
    let header_end = find_subsequence(part, b"\r\n\r\n")?;
    let headers = std::str::from_utf8(&part[..header_end]).ok()?;
    let body = &part[header_end + 4..];

    // Extract field name from Content-Disposition header
    for line in headers.lines() {
        if line.to_lowercase().starts_with("content-disposition:") {
            // Look for name="..."
            if let Some(name_start) = line.find("name=\"") {
                let name_start = name_start + 6;
                if let Some(name_end) = line[name_start..].find('"') {
                    let name = &line[name_start..name_start + name_end];

                    // Remove trailing CRLF from body if present
                    let body = if body.ends_with(b"\r\n") {
                        &body[..body.len() - 2]
                    } else {
                        body
                    };

                    return Some((name.to_string(), body));
                }
            }
        }
    }

    None
}

/// Finds a subsequence in a byte slice.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};

    // Test data: 1x1 red PNG pixel
    fn red_pixel_png() -> Vec<u8> {
        vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, // IHDR chunk length
            0x49, 0x48, 0x44, 0x52, // IHDR
            0x00, 0x00, 0x00, 0x01, // width = 1
            0x00, 0x00, 0x00, 0x01, // height = 1
            0x08, 0x02, // bit depth = 8, color type = RGB
            0x00, 0x00, 0x00, // compression, filter, interlace
            0x90, 0x77, 0x53, 0xDE, // CRC
            0x00, 0x00, 0x00, 0x0C, // IDAT chunk length
            0x49, 0x44, 0x41, 0x54, // IDAT
            0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x00, 0x03, 0x00,
            0x01, // compressed data
            0x00, 0x18, 0xDD, 0x8D, // CRC
            0x00, 0x00, 0x00, 0x00, // IEND chunk length
            0x49, 0x45, 0x4E, 0x44, // IEND
            0xAE, 0x42, 0x60, 0x82, // CRC
        ]
    }

    #[test]
    fn detect_png_format() {
        let png = red_pixel_png();
        assert_eq!(detect_image_format(&png), Some("image/png"));
    }

    #[test]
    fn detect_jpeg_format() {
        let jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(detect_image_format(&jpeg), Some("image/jpeg"));
    }

    #[test]
    fn detect_gif_format() {
        let gif = b"GIF89a\x01\x00\x01\x00";
        assert_eq!(detect_image_format(gif), Some("image/gif"));
    }

    #[test]
    fn detect_webp_format() {
        let webp = b"RIFF\x00\x00\x00\x00WEBP";
        assert_eq!(detect_image_format(webp), Some("image/webp"));
    }

    #[test]
    fn detect_unknown_format() {
        let data = b"not an image";
        assert_eq!(detect_image_format(data), None);
    }

    #[test]
    fn extract_openai_format() {
        let png = red_pixel_png();
        let b64 = STANDARD.encode(&png);
        let json = format!(r#"{{"data": [{{"b64_json": "{}"}}]}}"#, b64);

        let images = extract_images_from_json(json.as_bytes());
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].data, png);
        assert_eq!(images[0].source_path, "data[0].b64_json");
    }

    #[test]
    fn extract_stability_format() {
        let png = red_pixel_png();
        let b64 = STANDARD.encode(&png);
        let json = format!(r#"{{"artifacts": [{{"base64": "{}"}}]}}"#, b64);

        let images = extract_images_from_json(json.as_bytes());
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].data, png);
        assert_eq!(images[0].source_path, "artifacts[0].base64");
    }

    #[test]
    fn extract_xai_grok_format() {
        let png = red_pixel_png();
        let b64 = STANDARD.encode(&png);
        let json = format!(r#"{{"images": [{{"image": "{}"}}]}}"#, b64);

        let images = extract_images_from_json(json.as_bytes());
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].data, png);
        assert_eq!(images[0].source_path, "images[0].image");
    }

    #[test]
    fn extract_data_uri_format() {
        let png = red_pixel_png();
        let b64 = STANDARD.encode(&png);
        let data_uri = format!("data:image/png;base64,{}", b64);
        let json = format!(r#"{{"output": ["{}"]}}"#, data_uri);

        let images = extract_images_from_json(json.as_bytes());
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].data, png);
    }

    #[test]
    fn extract_multiple_images() {
        let png = red_pixel_png();
        let b64 = STANDARD.encode(&png);
        let json = format!(
            r#"{{"data": [{{"b64_json": "{}"}}, {{"b64_json": "{}"}}]}}"#,
            b64, b64
        );

        let images = extract_images_from_json(json.as_bytes());
        assert_eq!(images.len(), 2);
    }

    #[test]
    fn extract_binary_response() {
        let png = red_pixel_png();
        let img = extract_image_from_binary(&png, Some("image/png"));

        assert!(img.is_some());
        let img = img.unwrap();
        assert_eq!(img.data, png);
        assert_eq!(img.format, Some("image/png".to_string()));
    }

    #[test]
    fn extracted_image_is_valid() {
        let png = red_pixel_png();
        let img = ExtractedImage::new(png.clone(), "test", 0);
        assert!(img.is_valid_image());

        let invalid = ExtractedImage::new(b"not an image".to_vec(), "test", 0);
        assert!(!invalid.is_valid_image());
    }

    #[test]
    fn decode_base64_with_whitespace() {
        let png = red_pixel_png();
        let b64 = STANDARD.encode(&png);
        // Add whitespace and newlines
        let b64_with_ws = format!("{}\n{}", &b64[..10], &b64[10..]);

        let decoded = decode_base64_image(&b64_with_ws);
        assert!(decoded.is_some());
        assert_eq!(decoded.unwrap(), png);
    }

    #[test]
    fn invalid_json_returns_empty() {
        let images = extract_images_from_json(b"not json");
        assert!(images.is_empty());
    }

    #[test]
    fn no_images_returns_empty() {
        let json = r#"{"message": "Hello, world!"}"#;
        let images = extract_images_from_json(json.as_bytes());
        assert!(images.is_empty());
    }
}
