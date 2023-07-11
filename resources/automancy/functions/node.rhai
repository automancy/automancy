fn id_deps() {
    [
        ["target", "TARGET"],
        ["link", "LINK"],
    ]
}

fn function_id() {
    "node"
}

fn handle_transaction(input) {
    let target = this.data[TARGET];

    if type_of(target) != "()" {
        return ["proxy", input.coord + target];
    }
}

fn handle_tick(input) {
    let link = this.data[LINK];

    if type_of(link) != "()" {
        return ["make_extract_request", link, input.id, input.coord];
    }
}