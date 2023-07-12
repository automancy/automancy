fn id_deps() {
    [
        ["buffer", "BUFFER"],
        ["script", "SCRIPT"],
        ["target", "TARGET"],
    ]
}

fn function_id() {
    "machine"
}

fn handle_transaction(input) {
    let inputs = this.data[SCRIPT]?.as_script()?.instructions?.inputs;
    if inputs == () {
        return;
    }

    let matched = item_matches(input.stack.item.id, inputs);
    if matched == () {
        return;
    }

    let buffer = this.data.get_or_insert(BUFFER, "Inventory");
    
    let amount = matched.amount * 8;
    let stored = buffer[input.stack.item];
    if stored >= amount {
        return;
    }
    let inserting = min(input.stack.amount, amount - stored);

    buffer[input.stack.item] = stored + inserting;

    this.data[BUFFER] = buffer;

    return ["consume", inserting]
}

fn handle_tick(input) {
    let target = this.data[TARGET];

    if target == () {
        return;
    }

    let coord = input.coord + target;

    let script = this.data[SCRIPT]?.as_script();
    if script == () {
        return;
    }
    
    let outputs = script.instructions.outputs;
    let inputs = script.instructions.inputs;

    if inputs == () {
        return ["make_transaction", coord, input.id, input.coord, outputs];
    }

    let buffer = this.data[BUFFER];
    if buffer == () {
        return;
    }

    for input in inputs {
        let stored = buffer[input.item];

        if stored < input.amount {
            return;
        }
    }

    ["make_transaction", coord, input.id, input.coord, outputs]
}

fn handle_transaction_result(input) {
    let inputs = this.data[SCRIPT]?.as_script()?.instructions?.inputs;
    let buffer = this.data[BUFFER];

    if inputs != () && buffer != () {
        for input in inputs {
            let stored = buffer[input.item];

            if stored < input.amount {
                stored = 0;
                throw "in transaction result: tile does not have enough input for the supposed output!";
            } else {
                buffer[input.item] = stored - input.amount;
            }
        }

        this.data[BUFFER] = buffer;
    }
}