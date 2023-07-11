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
    let item = this.data[ITEM]?.as_item();
    let target = this.data[TARGET];

    if type_of(item) != "()" && type_of(target) != "()" {
        if input.stack.item == item {
            return ["pass_on", input.coord + target];
        } else {
            return ["pass_on", input.coord + target.rotate_right()];
        }
    }
}