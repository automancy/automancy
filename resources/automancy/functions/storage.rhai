fn id_deps() {
    [
        ["buffer", "BUFFER"],
        ["amount", "AMOUNT"],
        ["storage", "STORAGE"],
    ]
}

fn function_id() {
    "storage"
}

fn handle_transaction(input) {
    let item = this.data[STORAGE]?.id()?.as_item();
    let amount = this.data[AMOUNT]?.amount();
    let stack = input.stack;

    if type_of(amount) != "()" && type_of(item) != "()" && stack.item == item {
        let buffer = this.data.get_or_insert(BUFFER, "Inventory").inventory();

        let stored = buffer[item];
        if stored > amount {
            stored = amount;
        }

        if stored == amount {
            return;
        }

        let inserting = min(stack.amount, amount - stored);
        stored += inserting;

        buffer[item] = stored;
        this.data[BUFFER] = buffer;

        return ["consume"];
    }
}

fn handle_transaction_result(input) {
    let buffer = this.data[BUFFER]?.inventory();

    buffer.take(input.transferred.item, input.transferred.amount);
    
    this.data[BUFFER] = buffer;
}