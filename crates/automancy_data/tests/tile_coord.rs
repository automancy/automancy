use approx::assert_abs_diff_eq;
use automancy_data::{
    game::coord::{TileBounds, TileCoord},
    math::Float,
};

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

#[test]
fn test_tile_bounds_radial() {
    {
        let center = TileCoord::new(69, 88);
        let correct = [center];

        let bounds = TileBounds::radial(center, 1);
        let iterated = bounds.into_iter().collect::<Vec<_>>();

        assert_eq!(correct.as_slice(), iterated.as_slice());
    }

    {
        let center = TileCoord::new(0, 0);

        #[allow(clippy::identity_op)]
        let correct = [
            // ring 0
            center,
            // ring 1
            center + TileCoord::EDGES[0],
            center + TileCoord::EDGES[1],
            center + TileCoord::EDGES[2],
            center + TileCoord::EDGES[3],
            center + TileCoord::EDGES[4],
            center + TileCoord::EDGES[5],
            // ring 2
            center + TileCoord::EDGES[0] * 2,
            center + TileCoord::EDGES[0] * 2 + TileCoord::EDGES[(0 + 2) % 6],
            center + TileCoord::EDGES[1] * 2,
            center + TileCoord::EDGES[1] * 2 + TileCoord::EDGES[(1 + 2) % 6],
            center + TileCoord::EDGES[2] * 2,
            center + TileCoord::EDGES[2] * 2 + TileCoord::EDGES[(2 + 2) % 6],
            center + TileCoord::EDGES[3] * 2,
            center + TileCoord::EDGES[3] * 2 + TileCoord::EDGES[(3 + 2) % 6],
            center + TileCoord::EDGES[4] * 2,
            center + TileCoord::EDGES[4] * 2 + TileCoord::EDGES[(4 + 2) % 6],
            center + TileCoord::EDGES[5] * 2,
            center + TileCoord::EDGES[5] * 2 + TileCoord::EDGES[(5 + 2) % 6],
        ];

        let bounds = TileBounds::radial(center, 3);
        let iterated = bounds.into_iter().collect::<Vec<_>>();

        assert_eq!(correct.as_slice(), iterated.as_slice());
    }
}

#[test]
fn test_tile_bounds_rect() {
    let min = TileCoord::new(-269, -888);
    let max = TileCoord::new(269, 888);

    let correct = (min.r..max.r)
        .flat_map(|r| (min.q..max.q).map(move |q| TileCoord::new(q, r)))
        .collect::<Vec<_>>();

    let bounds = TileBounds::rect(min, max);
    let iterated = bounds.into_iter().collect::<Vec<_>>();

    assert_eq!(correct.as_slice(), iterated.as_slice());
}
