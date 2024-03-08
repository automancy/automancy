fn id_deps() {
    [
        ["item", "ITEM"],
        ["target", "TARGET"],
    ]
}

fn function_id() {
    "sorter"
}

fn handle_transaction(input) {
    let item = this[ITEM]?.as_item();
    let target = this[TARGET];

    if item != () && target != () {
        if input.stack.item == item {
            return ["pass_on", input.coord + target];
        } else {
            return ["pass_on", input.coord + target.rotate_right()];
        }
    }
}