//! The single-bitmap raster rule: the checks run in order and the first failure names the call.

use super::check_raster;
use crate::error::{Error, LayerCall};

const CALL: LayerCall = LayerCall::ViewshedUpload;

#[test]
fn a_tight_aligned_raster_passes() {
    assert_eq!(check_raster(CALL, 64, 3, 256, 768), Ok(()));
}

#[test]
fn a_padded_row_passes() {
    assert_eq!(check_raster(CALL, 10, 2, 256, 512), Ok(()));
}

#[test]
fn a_zero_dimension_is_refused_first() {
    assert_eq!(
        check_raster(CALL, 0, 3, 7, 1),
        Err(Error::ZeroTextureDimensions { call: CALL })
    );
    assert_eq!(
        check_raster(CALL, 3, 0, 7, 1),
        Err(Error::ZeroTextureDimensions { call: CALL })
    );
}

#[test]
fn a_row_shorter_than_four_bytes_per_texel_is_refused() {
    assert_eq!(
        check_raster(CALL, 65, 1, 256, 256),
        Err(Error::RasterRowPitch {
            call: CALL,
            bytes_per_row: 256,
            width: 65,
        })
    );
}

#[test]
fn an_unaligned_row_is_refused() {
    assert_eq!(
        check_raster(CALL, 10, 1, 40, 40),
        Err(Error::RasterRowPitch {
            call: CALL,
            bytes_per_row: 40,
            width: 10,
        })
    );
}

#[test]
fn a_width_whose_row_overflows_u32_is_refused_not_wrapped() {
    assert_eq!(
        check_raster(CALL, u32::MAX, 1, 256, 256),
        Err(Error::RasterRowPitch {
            call: CALL,
            bytes_per_row: 256,
            width: u32::MAX,
        })
    );
}

#[test]
fn a_length_other_than_rows_times_pitch_is_refused() {
    assert_eq!(
        check_raster(LayerCall::ForestDensityUpload, 64, 2, 256, 511),
        Err(Error::RasterByteLength {
            call: LayerCall::ForestDensityUpload,
            expected: 512,
            actual: 511,
        })
    );
}
