fn id_deps() {
    []
}

fn function_id() {
    "void"
}

fn handle_transaction(input) {
    return [TransResult::CONSUME, input.stack.amount];
}