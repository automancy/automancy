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
    let storage = this.data[STORAGE]?.id()?.as_item();
    let amount = this.data[AMOUNT]?.amount();

    if type_of(amount) != "()" && type_of(storage) != "()" && input.stack.item == storage {
        let buffer = this.data.get_or_insert(BUFFER, "Inventory").inventory();

        let stored = buffer[storage];
        if stored > amount {
            stored = amount;
        }

        if stored == amount {
            return;
        }

        let inserting = min(input.stack.amount, amount - stored);
        stored += inserting;

        buffer[storage] = stored;
        this.data[BUFFER] = buffer;

        return ["consume", inserting];
    }
}

fn handle_transaction_result(input) {
    let buffer = this.data[BUFFER]?.inventory();

    buffer.take(input.transferred.item, input.transferred.amount);
    
    this.data[BUFFER] = buffer;
}

fn handle_extract_request(input) {
    let storage = this.data[STORAGE]?.id().as_item();
    let amount = this.data[AMOUNT]?.amount();
    let buffer = this.data[BUFFER]?.inventory();

    if type_of(storage) != "()" && type_of(amount) != "()" && type_of(buffer) != "()" {
        let stored = buffer[storage];
        //let extracting = min(stored, amount); TODO ????

        if stored > 0 {
            return [
                "make_transaction",
                input.requested_from_coord,
                input.id,
                input.coord,
                [ItemStack(storage, stored)]
            ];
        }
    }
}