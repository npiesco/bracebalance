-- Deeply nested Lua - HAS ISSUES (missing closers)
-- Line comment trap: { [ ( } ] )
--[[ Block comment trap: { [ ( } ] ) ]]
--[==[ Level-2 trap: { [ ( } ] ) ]==]

local _trap1 = "string trap: { [ ( } ] )"
local _trap2 = [[long string trap: { [ ( } ] )]]

local function broken()
    local data = {
        level1 = {
            level2 = {
                level3 = {
                    level4 = {
                        values = {1, 2, {3, {4, 5}},  -- <-- missing }
                    },
                },
            },
        },
    }

    local result = (function()
        return {
            items = {
                {name = "a", vals = {1, {2, 3}},
                {name = "b", vals = {4, {5, 6}}},  -- <-- missing } for first item
            },
        }
    end)()

    local nested = {
        {1, {2, {3, {4, {5,  -- <-- missing 4 closing }
    }

    return data
end

return {broken = broken}
