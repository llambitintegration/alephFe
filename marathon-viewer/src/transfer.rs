//! Marathon transfer mode constants.
//!
//! Values mirror Alephone's `map.h` transfer-mode enum (all 28 modes).
#![allow(dead_code)]

pub const TRANSFER_NORMAL: u32 = 0;
pub const TRANSFER_FADE_OUT_TO_BLACK: u32 = 1;
pub const TRANSFER_INVISIBILITY: u32 = 2;
pub const TRANSFER_SUBTLE_INVISIBILITY: u32 = 3;
pub const TRANSFER_PULSATE: u32 = 4;
pub const TRANSFER_WOBBLE: u32 = 5;
pub const TRANSFER_FAST_WOBBLE: u32 = 6;
pub const TRANSFER_STATIC: u32 = 7;
pub const TRANSFER_FIFTY_PERCENT_STATIC: u32 = 8;
pub const TRANSFER_LANDSCAPE: u32 = 9;
pub const TRANSFER_SMEAR: u32 = 10;
pub const TRANSFER_FADE_OUT_STATIC: u32 = 11;
pub const TRANSFER_PULSATING_STATIC: u32 = 12;
pub const TRANSFER_FOLD_IN: u32 = 13;
pub const TRANSFER_FOLD_OUT: u32 = 14;
pub const TRANSFER_HORIZONTAL_SLIDE: u32 = 15;
pub const TRANSFER_FAST_HORIZONTAL_SLIDE: u32 = 16;
pub const TRANSFER_VERTICAL_SLIDE: u32 = 17;
pub const TRANSFER_FAST_VERTICAL_SLIDE: u32 = 18;
pub const TRANSFER_WANDER: u32 = 19;
pub const TRANSFER_FAST_WANDER: u32 = 20;
pub const TRANSFER_BIG_LANDSCAPE: u32 = 21;
pub const TRANSFER_REVERSE_HORIZONTAL_SLIDE: u32 = 22;
pub const TRANSFER_REVERSE_FAST_HORIZONTAL_SLIDE: u32 = 23;
pub const TRANSFER_REVERSE_VERTICAL_SLIDE: u32 = 24;
pub const TRANSFER_REVERSE_FAST_VERTICAL_SLIDE: u32 = 25;
pub const TRANSFER_2X: u32 = 26;
pub const TRANSFER_4X: u32 = 27;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrected_constant_values_match_alephone() {
        // Previously-wrong constants now corrected.
        assert_eq!(TRANSFER_PULSATE, 4);
        assert_eq!(TRANSFER_WOBBLE, 5);
        assert_eq!(TRANSFER_STATIC, 7);
        assert_eq!(TRANSFER_HORIZONTAL_SLIDE, 15);

        // Previously-correct constants unchanged.
        assert_eq!(TRANSFER_NORMAL, 0);
        assert_eq!(TRANSFER_LANDSCAPE, 9);

        // A sample of the newly-added constants.
        assert_eq!(TRANSFER_FAST_WOBBLE, 6);
        assert_eq!(TRANSFER_FIFTY_PERCENT_STATIC, 8);
        assert_eq!(TRANSFER_VERTICAL_SLIDE, 17);
        assert_eq!(TRANSFER_4X, 27);
    }
}
