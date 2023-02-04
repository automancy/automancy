use automancy::game::data::TileCoord;

#[test]
fn test_tile_coord_serde() {
    let c = TileCoord::new(123, 456);
    println!("{c:?}");

    let serialized = serde_json::to_string(&c).unwrap();
    println!("{serialized}");

    let deserialized: TileCoord = serde_json::from_str(&serialized).unwrap();
    assert_eq!(c, deserialized);
}
