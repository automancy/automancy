fn id_deps() {
    [
        ["target", "TARGET"],
    ]
}

fn function_id() {
    "master_node"
}

fn handle_extract_request(input) {
    let target = this.data[TARGET];

    if type_of(target) != "()" {
        return ["make_extract_request", input.coord + target, input.requested_from_id, input.requested_from_coord]
    }
}