use binrw::BinRead;

/// Convert a Marathon 16.16 fixed-point value to f32.
pub fn fixed_to_f32(v: i32) -> f32 {
    v as f32 / 65536.0
}

/// Convert a Marathon world_distance (i16, 1024 = 1 world unit) to f32.
pub fn world_distance_to_f32(v: i16) -> f32 {
    v as f32 / 1024.0
}

/// Pack four ASCII characters into a big-endian u32 tag code.
pub const fn four_chars(a: u8, b: u8, c: u8, d: u8) -> u32 {
    ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32)
}

/// World-space 2D point. Coordinates are in world units (i16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct WorldPoint2d {
    pub x: i16,
    pub y: i16,
}

/// World-space 3D point. Coordinates are in world units (i16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct WorldPoint3d {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

/// Marathon angle newtype. Full circle = 512 units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct MarathonAngle(pub i16);

impl MarathonAngle {
    pub fn to_radians(self) -> f32 {
        self.0 as f32 * (std::f32::consts::TAU / 512.0)
    }

    pub fn to_degrees(self) -> f32 {
        self.0 as f32 * (360.0 / 512.0)
    }
}

/// Shape descriptor: encodes collection index, CLUT, and shape index in a u16.
/// Bits [12:8] = collection (0-31), bits [15:13] = CLUT (0-7), bits [7:0] = shape index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct ShapeDescriptor(pub u16);

impl ShapeDescriptor {
    pub fn collection(&self) -> u8 {
        ((self.0 >> 8) & 0x1F) as u8
    }

    pub fn clut(&self) -> u8 {
        ((self.0 >> 13) & 0x07) as u8
    }

    pub fn shape_index(&self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    pub fn is_none(&self) -> bool {
        self.0 == 0xFFFF
    }

    pub fn from_parts(collection: u8, clut: u8, shape_index: u8) -> Self {
        debug_assert!(collection < 32);
        debug_assert!(clut < 8);
        Self(((clut as u16 & 0x07) << 13) | ((collection as u16 & 0x1F) << 8) | shape_index as u16)
    }
}

/// Texture definition used by sides (x/y offset + shape descriptor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct SideTexture {
    pub x0: i16,
    pub y0: i16,
    pub texture: ShapeDescriptor,
}

/// Damage definition used in physics and map data.
#[derive(Debug, Clone, Copy, PartialEq, BinRead)]
#[br(big)]
pub struct DamageDefinition {
    pub damage_type: i16,
    pub flags: i16,
    pub base: i16,
    pub random: i16,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub scale: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_types_fixed_to_f32() {
        assert_eq!(fixed_to_f32(0x10000), 1.0);
        assert_eq!(fixed_to_f32(0x8000), 0.5);
        assert_eq!(fixed_to_f32(0), 0.0);
        // -1.0 in 16.16 fixed point
        assert_eq!(fixed_to_f32(-0x10000_i32), -1.0);
    }

    #[test]
    fn test_types_world_distance_to_f32() {
        assert_eq!(world_distance_to_f32(1024), 1.0);
        assert_eq!(world_distance_to_f32(512), 0.5);
        assert_eq!(world_distance_to_f32(0), 0.0);
    }

    #[test]
    fn test_types_marathon_angle() {
        let a = MarathonAngle(128); // quarter turn
        assert!((a.to_degrees() - 90.0).abs() < 0.01);
        assert!((a.to_radians() - std::f32::consts::FRAC_PI_2).abs() < 0.01);

        let full = MarathonAngle(512);
        assert!((full.to_degrees() - 360.0).abs() < 0.01);
    }

    #[test]
    fn test_types_four_chars() {
        assert_eq!(four_chars(b'P', b'N', b'T', b'S'), 0x504E5453);
        assert_eq!(four_chars(b'L', b'I', b'N', b'S'), 0x4C494E53);
    }

    #[test]
    fn test_types_shape_descriptor_roundtrip() {
        // collection=5, clut=2, shape=42
        let sd = ShapeDescriptor::from_parts(5, 2, 42);
        assert_eq!(sd.collection(), 5);
        assert_eq!(sd.clut(), 2);
        assert_eq!(sd.shape_index(), 42);
        assert!(!sd.is_none());

        let none = ShapeDescriptor(0xFFFF);
        assert!(none.is_none());
    }
}
