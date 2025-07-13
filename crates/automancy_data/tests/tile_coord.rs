use approx::assert_abs_diff_eq;
use automancy_data::{game::coord::TileCoord, math::Float};

#[test]
fn test_coord_to_angle() {
    fn control_result(coord: TileCoord) -> Float {
        TileCoord::world_pos_to_degrees(coord.to_world_pos())
    }

    #[track_caller]
    fn test(coord: TileCoord) {
        assert_abs_diff_eq!(control_result(coord), coord.as_degrees())
    }

    #[track_caller]
    fn test2(a: TileCoord, b: TileCoord) {
        assert_abs_diff_eq!(control_result(a), b.as_degrees())
    }

    test(TileCoord::new(69, 420));

    test(TileCoord::TOP_RIGHT);
    test(TileCoord::TOP_LEFT);
    test(TileCoord::LEFT);
    test(TileCoord::BOTTOM_LEFT);
    test(TileCoord::BOTTOM_RIGHT);
    test(TileCoord::RIGHT);

    test2(30 * TileCoord::TOP_RIGHT, TileCoord::TOP_RIGHT);
    test2(30 * TileCoord::TOP_LEFT, TileCoord::TOP_LEFT);
    test2(30 * TileCoord::LEFT, TileCoord::LEFT);
    test2(30 * TileCoord::BOTTOM_LEFT, TileCoord::BOTTOM_LEFT);
    test2(30 * TileCoord::BOTTOM_RIGHT, TileCoord::BOTTOM_RIGHT);
    test2(30 * TileCoord::RIGHT, TileCoord::RIGHT);
}

#[test]
fn test_tile_coord_serde() {
    let c = TileCoord::new(123, 456);

    let serialized = ron::to_string(&c).unwrap();

    let deserialized: TileCoord = ron::from_str(&serialized).unwrap();

    assert_eq!(c, deserialized);
}
