fn id_deps() {
    [
        ["target", "TARGET"],
    ]
}

fn function_id() {
    "merger"
}

fn handle_transaction(input) {
    let target = this.data[TARGET]?.coord();
    
    if type_of(target) != "()" {
        return ["pass_on", input.coord + target];
    }
}