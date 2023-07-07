fn function_id() {
    "merger"
}

fn handle_transaction(input) {
    let target = this.data.target?.coord();
    
    if type_of(target) == "TileCoord" {
        let target_coord = input.coord + target;

        return ["pass_on", target_coord];
    }
}