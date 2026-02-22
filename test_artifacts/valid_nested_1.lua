-- Deeply nested Lua - BALANCED
-- Line comment trap: { [ ( } ] )
--[[ Block comment trap: { [ ( } ] ) ]]
--[==[ Level-2 block trap: { [ ( } ] ) ]==]

local _trap1 = "string trap: { [ ( } ] )"
local _trap2 = [[
  long string trap: { [ ( } ] )
  more traps: vec![ { ( ]
]]
local _trap3 = [==[
  level-2 long string: { [ ( } ] )
]==]

local function deep_config()
    local config = {
        database = {
            connections = {
                {
                    primary = {
                        host = "localhost",
                        options = {
                            pool = {
                                min = {2, 4},
                                max = {10, 20},
                                settings = {
                                    {timeout = {30, {60, 90}}},
                                    {retry   = {1,  {2,  3}}},
                                },
                            },
                        },
                    },
                    replicas = {
                        {
                            host = "replica1",
                            weights = {
                                {1, 2, 3},
                                {4, 5, 6},
                                {
                                    computed = {
                                        {math.max(1,2), math.min(3,4)},
                                        {math.max(5,6), math.min(7,8)},
                                    },
                                },
                            },
                        },
                    },
                },
            },
        },
        cache = {
            layers = {
                {"L1", {size = {1024, {2048, {unit = "KB"}}}}},
                {"L2", {size = {4096, {8192, {unit = "MB"}}}}},
            },
        },
    }
    return config
end

local function process(input)
    local result = {}
    for k, v in pairs(input) do
        result[k] = {
            transformed = (function()
                if type(v) == "table" then
                    local inner = {}
                    for i, item in ipairs(v) do
                        inner[i] = {
                            index = i,
                            value = item,
                            meta  = {[[ ignored ]], type(item)},
                        }
                    end
                    return inner
                end
                return {raw = v}
            end)(),
        }
    end
    return result
end

return {deep_config = deep_config, process = process}
