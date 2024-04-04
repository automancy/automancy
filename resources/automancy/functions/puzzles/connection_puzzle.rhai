fn id_deps() {
    [
        ["tiles", "TILES"],
        ["anchors", "ANCHORS"],
        ["selections", "SELECTIONS"],
        ["bounds", "BOUNDS"],
        ["connections", "CONNECTIONS"],
        ["core:puzzle_space", "SPACE"],
    ]
}

fn function_id() {
    "puzzles/connection_puzzle"
}

fn pre_setup(setup) {
    let anchors = setup[ANCHORS];

    let tiles = TileMap();
    let anchors_coords = anchors.keys();
    for coord in anchors_coords {
        tiles[coord] = anchors[coord];
    }

    let bounds = TileBounds(anchors);
    this[BOUNDS] = bounds;

    for coord in bounds {
        if coord !in anchors_coords {
            tiles[coord] = SPACE;
        }
    }

    this[TILES] = tiles;
}

fn selection_at_coord(setup, coord) {
    if coord in setup[ANCHORS].keys() {
        return;
    }

    return setup[SELECTIONS];
}

fn bfs(tiles, connections, checked, to_check) {
    let check_next = [];

    for coord in to_check {
        let id = tiles[coord];
        let neighbors = coord.neighbors();

        for neighbor in neighbors {
            if neighbor in checked {
                continue;
            }
            if !(neighbor in tiles) {
                continue;
            }

            let neighbor_id = tiles[neighbor];
            let forw = connections[neighbor_id] ?? [];
            let back = connections[id] ?? [];
            if (id in forw) || (neighbor_id in back) {
                check_next += neighbor;
                checked += coord;
                checked += neighbor;
            }
        }
    }

    if check_next.is_empty() {
        return ();
    }
    return [checked, check_next];
}

fn evaluate(setup) {
    let connections = setup[CONNECTIONS];
    let tiles = this[TILES];

    let checked = [];
    let to_check = tiles.keys();

    loop {
        let result = bfs(tiles, connections, checked, to_check);
        if result == () {
            break;
        }
        checked += result[0];
        to_check = result[1];
    }

    for coord in setup[ANCHORS].keys() {
        if !(coord in checked) {
            return false;
        }
    }

    return true;
}