fn id_deps() {
    []
}

fn function_id() {
    "splitter_left"
}

fn handle_transaction(input) {
    let dirs = [
       TileCoord::TOP_RIGHT,
       TileCoord::BOTTOM_RIGHT,
       TileCoord::LEFT,
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