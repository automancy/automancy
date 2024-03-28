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
        return [Result::MAKE_EXTRACT_REQUEST, link, input.id, input.coord];
    }
}

fn handle_transaction(input) {
    let target = this[TARGET];

    if target != () {
        return [TransResult::PASS_ON, input.coord + target];
    }
}