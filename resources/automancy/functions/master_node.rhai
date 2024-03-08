fn id_deps() {
    [
        ["target", "TARGET"],
        ["requested_from_coord", "REQUESTED_FROM_COORD"],
    ]
}

fn function_id() {
    "master_node"
}

fn handle_transaction(input) {
    let requested_from_coord = this[REQUESTED_FROM_COORD];

    if requested_from_coord != () {
        return ["pass_on", requested_from_coord]
    }
}

fn handle_extract_request(input) {
    let target = this[TARGET];

    this[REQUESTED_FROM_COORD] = input.requested_from_coord;

    if target != () {
        return ["make_extract_request", input.coord + target, input.id, input.coord]
    }
}