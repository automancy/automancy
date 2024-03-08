fn id_deps() {
    [
        ["target", "TARGET"],
        ["link", "LINK"],
    ]
}

fn function_id() {
    "node"
}

fn handle_tick(input) {
    let link = this[LINK];

    if link != () {
        return ["make_extract_request", link, input.id, input.coord];
    }
}

fn handle_transaction(input) {
    let target = this[TARGET];

    if target != () {
        return ["pass_on", input.coord + target];
    }
}