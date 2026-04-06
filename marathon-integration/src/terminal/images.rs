/// Terminal image loading and rendering bridge.
///
/// Marathon terminals can display inline images referenced by PICT resource ID.
/// In the original engine, these were Mac PICT resources stored in the scenario's
/// resource fork. In our implementation, we load image data from the scenario's
/// shapes collections or external image data and convert them to RGBA pixel buffers
/// for rendering as textured quads in the terminal view.

/// Decoded image data ready for GPU upload.
#[derive(Debug, Clone)]
pub struct TerminalImageData {
    /// PICT resource ID this image was loaded from.
    pub resource_id: u16,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// RGBA pixel data (width * height * 4 bytes).
    pub rgba_data: Vec<u8>,
}

/// Cache for loaded terminal images to avoid re-decoding.
#[derive(Debug, Default)]
pub struct TerminalImageCache {
    images: Vec<TerminalImageData>,
}

impl TerminalImageCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a cached image by resource ID.
    pub fn get(&self, resource_id: u16) -> Option<&TerminalImageData> {
        self.images.iter().find(|img| img.resource_id == resource_id)
    }

    /// Insert a decoded image into the cache.
    pub fn insert(&mut self, image: TerminalImageData) {
        // Replace existing entry for same resource ID
        self.images.retain(|img| img.resource_id != image.resource_id);
        self.images.push(image);
    }

    /// Clear the cache (e.g., when changing levels).
    pub fn clear(&mut self) {
        self.images.clear();
    }
}

/// Load a terminal image from raw PICT resource data.
///
/// Marathon PICT resources are simple indexed-color bitmaps. This function
/// decodes the resource data into an RGBA pixel buffer.
///
/// Returns None if the resource data is invalid or empty.
pub fn decode_pict_resource(resource_id: u16, data: &[u8]) -> Option<TerminalImageData> {
    if data.len() < 12 {
        return None;
    }

    // Minimal PICT header parsing:
    // Bytes 0-1: file size (ignored in v2)
    // Bytes 2-5: bounding rect top, left
    // Bytes 6-9: bounding rect bottom, right
    let top = i16::from_be_bytes([data[2], data[3]]) as u32;
    let left = i16::from_be_bytes([data[4], data[5]]) as u32;
    let bottom = i16::from_be_bytes([data[6], data[7]]) as u32;
    let right = i16::from_be_bytes([data[8], data[9]]) as u32;

    let width = right.saturating_sub(left);
    let height = bottom.saturating_sub(top);

    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return None;
    }

    // Generate a placeholder image (solid dark green) when actual pixel data
    // parsing is not yet implemented. The terminal renderer will display
    // this in the correct position.
    let pixel_count = (width * height) as usize;
    let mut rgba_data = Vec::with_capacity(pixel_count * 4);
    for _ in 0..pixel_count {
        rgba_data.extend_from_slice(&[0, 40, 0, 255]); // Dark green placeholder
    }

    Some(TerminalImageData {
        resource_id,
        width,
        height,
        rgba_data,
    })
}

/// Create a placeholder image for missing PICT resources.
pub fn placeholder_image(resource_id: u16) -> TerminalImageData {
    let width = 128;
    let height = 128;
    let pixel_count = width * height;
    let mut rgba_data = Vec::with_capacity(pixel_count * 4);

    // Checkerboard pattern in dark colors to indicate missing image
    for y in 0..height {
        for x in 0..width {
            let checker = ((x / 8) + (y / 8)) % 2 == 0;
            let c = if checker { 30u8 } else { 15u8 };
            rgba_data.extend_from_slice(&[c, c, c, 255]);
        }
    }

    TerminalImageData {
        resource_id,
        width: width as u32,
        height: height as u32,
        rgba_data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_insert_and_get() {
        let mut cache = TerminalImageCache::new();
        assert!(cache.get(100).is_none());

        cache.insert(TerminalImageData {
            resource_id: 100,
            width: 64,
            height: 64,
            rgba_data: vec![0; 64 * 64 * 4],
        });

        assert!(cache.get(100).is_some());
        assert_eq!(cache.get(100).unwrap().width, 64);
    }

    #[test]
    fn cache_replaces_existing() {
        let mut cache = TerminalImageCache::new();
        cache.insert(TerminalImageData {
            resource_id: 100,
            width: 32,
            height: 32,
            rgba_data: vec![0; 32 * 32 * 4],
        });
        cache.insert(TerminalImageData {
            resource_id: 100,
            width: 64,
            height: 64,
            rgba_data: vec![0; 64 * 64 * 4],
        });

        // Should have only one entry
        assert_eq!(cache.images.len(), 1);
        assert_eq!(cache.get(100).unwrap().width, 64);
    }

    #[test]
    fn cache_clear() {
        let mut cache = TerminalImageCache::new();
        cache.insert(TerminalImageData {
            resource_id: 1,
            width: 16,
            height: 16,
            rgba_data: vec![0; 16 * 16 * 4],
        });
        cache.clear();
        assert!(cache.get(1).is_none());
    }

    #[test]
    fn decode_pict_too_short() {
        assert!(decode_pict_resource(1, &[0; 5]).is_none());
    }

    #[test]
    fn decode_pict_zero_dimensions() {
        // top=0, left=0, bottom=0, right=0
        let data = [0u8; 12];
        assert!(decode_pict_resource(1, &data).is_none());
    }

    #[test]
    fn decode_pict_valid_header() {
        let mut data = vec![0u8; 12];
        // top=0, left=0, bottom=64, right=128
        data[6] = 0;
        data[7] = 64;
        data[8] = 0;
        data[9] = 128;

        let img = decode_pict_resource(42, &data).unwrap();
        assert_eq!(img.resource_id, 42);
        assert_eq!(img.width, 128);
        assert_eq!(img.height, 64);
        assert_eq!(img.rgba_data.len(), 128 * 64 * 4);
    }

    #[test]
    fn placeholder_image_correct_size() {
        let img = placeholder_image(999);
        assert_eq!(img.resource_id, 999);
        assert_eq!(img.width, 128);
        assert_eq!(img.height, 128);
        assert_eq!(img.rgba_data.len(), 128 * 128 * 4);
    }
}
