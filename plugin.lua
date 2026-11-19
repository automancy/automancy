---Mirrors the behavior of `automancy_game::scripting_lua::module::register_module_loader`
---@param  workspace  string # The URI of the workspace
---@param  name       string # Argument of require()
---@return string[]?
function ResolveRequire(workspace, name)
    ---@type string[]
    local split_path = {}
    for str in string.gmatch(name, "([^.]+)") do
        table.insert(split_path, str)
    end

    local ty = split_path[1]
    local namespace = split_path[2]
    local file_name = split_path[3]

    if ty == nil or namespace == nil or file_name == nil then
        return
    end

    return { workspace .. "/resources/" .. namespace .. "/scripts/" .. ty .. "/" .. file_name .. ".lua" }
end
