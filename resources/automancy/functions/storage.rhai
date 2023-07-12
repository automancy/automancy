fn id_deps() {
    [
        ["buffer", "BUFFER"],
        ["amount", "AMOUNT"],
        ["item", "ITEM"],
    ]
}

fn function_id() {
    "storage"
}

fn handle_transaction(input) {
    let item = this.data[ITEM]?.as_item();
    let amount = this.data[AMOUNT];

    if amount != () && item != () && input.stack.item == item {
        let buffer = this.data.get_or_insert(BUFFER, "Inventory");

        let stored = buffer[item];
        if stored > amount {
            stored = amount;
        }

        if stored == amount {
            return;
        }

        let inserting = min(input.stack.amount, amount - stored);
        stored += inserting;

        buffer[item] = stored;
        this.data[BUFFER] = buffer;

        return ["consume", inserting];
    }
}

fn handle_transaction_result(input) {
    let buffer = this.data[BUFFER];

    buffer.take(input.transferred.item, input.transferred.amount);
    
    this.data[BUFFER] = buffer;
}

fn handle_extract_request(input) {
    let item = this.data[ITEM]?.as_item();
    let buffer = this.data[BUFFER];

    if item != () && buffer != () {
        let stored = buffer[item];

        if stored > 0 {
            return [
                "make_transaction",
                input.requested_from_coord,
                input.id,
                input.coord,
                [ItemStack(item, stored)]
            ];
        }
    }
}