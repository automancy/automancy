fn id_deps() {
    []
}

fn function_id() {
    "splitter"
}

fn handle_transaction(input) {
    let dirs = if input.tile_modifier == 0 {
        [
            TileCoord::TOP_LEFT,
            TileCoord::BOTTOM_LEFT,
            TileCoord::RIGHT
        ]
    } else {
        [
            TileCoord::TOP_RIGHT,
            TileCoord::BOTTOM_RIGHT,
            TileCoord::LEFT,
        ]
    };

    let direction = input.source_coord - input.coord;

    let choices;
    if direction == dirs[0] {
        choices = [dirs[1], dirs[2]]
    } else if direction == dirs[1] {
        choices = [dirs[0], dirs[2]]
    } else {
        choices = [dirs[0], dirs[1]]
    };

    let target = if input.random % 2 == 0 { choices[0] } else { choices[1] };

    ["pass_on", input.coord + target]
}