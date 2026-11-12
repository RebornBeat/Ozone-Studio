-- Luau companion runtime entrypoint.
-- Require this module from the Studio plugin bootstrap and call start().

local Runtime = require(script.Parent.runtime_loop)

return {
    name = "gamedev-all-in-one-runtime",
    version = "0.1.0",
    protocolVersion = 1,
    bridgeMode = "http-long-poll",
    start = Runtime.start,
}
