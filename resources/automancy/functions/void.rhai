fn id_deps() {
    []
}

fn function_id() {
    "void"
}

fn handle_transaction(input) {
    return ["consume", input.stack.amount];
}