fn id_deps() {
    []
}

fn function_id() {
    "splitter_right"
}

fn handle_transaction(input) {
    let dirs = [
       TileCoord::TOP_LEFT,
       TileCoord::BOTTOM_LEFT,
       TileCoord::RIGHT
   ];

    let direction = input.source_coord - input.coord;

    let choices;
    if direction == dirs[0] {
        choices = [dirs[1], dirs[2]]
    } else if direction == dirs[1] {
        choices = [dirs[0], dirs[2]]
    } else if direction == dirs[2] {
        choices = [dirs[0], dirs[1]]
    } else {
        return;
    }

    let target = if input.random % 2 == 0 { choices[0] } else { choices[1] };

    ["pass_on", input.coord + target]
}