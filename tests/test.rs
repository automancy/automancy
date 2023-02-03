use automancy::game::tile::directions::*;
use automancy::game::tile::TileCoord;

#[test]
fn test_tile_coord_serde() {
    let c = TileCoord::new(123, 456);
    println!("{c:?}");

    let serialized = serde_json::to_string(&c).unwrap();
    println!("{serialized}");

    let deserialized: TileCoord = serde_json::from_str(&serialized).unwrap();
    assert_eq!(c, deserialized);
}

#[test]
fn test_hex_dir() {
    println!("{TOP_RIGHT:?}");
    println!("{RIGHT:?}");
    println!("{BOTTOM_RIGHT:?}");
    println!("{BOTTOM_LEFT:?}");
    println!("{LEFT:?}");
    println!("{TOP_LEFT:?}");
}
