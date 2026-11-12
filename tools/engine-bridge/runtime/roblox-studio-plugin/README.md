# Roblox Studio Plugin Runtime

This directory is reserved for the Luau companion plugin/runtime side that lives with Roblox Studio.

The MCP server remains the public stdio surface.

This runtime layer is responsible for:

- executing Studio-side Luau actions
- returning logs and runtime feedback
- exposing playtest-oriented hooks
- acting as the local companion boundary for Roblox mutation workflows
