--[[
  Gamedev All-in-One MCP - Roblox Studio Plugin (Single File)
  
  INSTALLATION:
  1. Open Roblox Studio
  2. Go to View > Explorer if not visible
  3. Right-click ServerStorage (or any service) > Insert Object > Script
  4. Change the script type to "Plugin" or place this file in your local Plugins folder:
     - Windows: %LOCALAPPDATA%\Roblox\Plugins\
     - Mac: ~/Documents/Roblox/Plugins/
  5. Paste this entire file contents
  6. Restart Studio or click the "Runtime" button in the Plugins toolbar
  
  REQUIREMENTS:
  - Game Settings > Security > Allow HTTP Requests = ON
  - MCP server running: npm run dev (or npm start)
  
  BRIDGE URL: http://127.0.0.1:3002 (configurable below)
]]

local HttpService = game:GetService("HttpService")

local CONFIG = {
    runtimeName = "gamedev-all-in-one-runtime",
    runtimeVersion = "0.1.0",
    protocolVersion = 1,
    bridgeMode = "http-long-poll",
    baseUrl = "http://127.0.0.1:3002",
    commandTimeoutMs = 25000,
}

local running = false

local function jsonEncode(value)
    return HttpService:JSONEncode(value)
end

local function jsonDecode(value)
    return HttpService:JSONDecode(value)
end

local function request(method, url, body)
    local ok, response = pcall(function()
        return HttpService:RequestAsync({
            Url = url,
            Method = method,
            Headers = { ["Content-Type"] = "application/json" },
            Body = body and jsonEncode(body) or nil,
        })
    end)

    if not ok then
        warn("[MCP] Request failed: " .. tostring(response))
        return nil
    end

    if not response.Success then
        warn("[MCP] HTTP " .. tostring(response.StatusCode) .. ": " .. tostring(response.StatusMessage))
        return nil
    end

    if response.Body == "" then return nil end
    local decodeOk, decoded = pcall(jsonDecode, response.Body)
    if not decodeOk then return nil end
    return decoded
end

local function postHandshake()
    return request("POST", CONFIG.baseUrl .. "/runtime/handshake", {
        protocolVersion = CONFIG.protocolVersion,
        runtimeName = CONFIG.runtimeName,
        runtimeVersion = CONFIG.runtimeVersion,
        bridgeMode = CONFIG.bridgeMode,
        capabilities = {
            "run_code", "create_workspace_part",
            "get_script_source", "set_script_source", "edit_script_lines", "grep_scripts",
            "create_instance", "delete_instance", "set_property", "clone_instance", "reparent_instance",
            "get_instance_properties", "get_instance_children", "search_instances", "get_file_tree",
            "set_gravity", "set_physics", "add_constraint", "raycast", "simulate_physics",
        },
        lastSeenAt = DateTime.now():ToIsoDate(),
    })
end

local function submitResult(requestId, status, payload)
    return request("POST", CONFIG.baseUrl .. "/runtime/commands/result", {
        requestId = requestId,
        status = status,
        completedAt = DateTime.now():ToIsoDate(),
        output = payload.output,
        data = payload.data,
        error = payload.error,
    })
end

local function resolvePath(pathStr)
    local current = game
    for segment in string.gmatch(pathStr, "[^%.]+") do
        if current == game and segment == "game" then continue end
        current = current:FindFirstChild(segment)
        if not current then
            return nil, "Instance not found at segment: " .. segment .. " in path: " .. pathStr
        end
    end
    return current, nil
end

local function handleRunCode(payload)
    local fn, compileError = loadstring(payload.code)
    if not fn then return false, { error = compileError or "Failed to compile Luau code." } end
    local ok, result = pcall(fn)
    if not ok then return false, { error = tostring(result) } end
    return true, { output = tostring(result) }
end

local function handleCreateWorkspacePart(payload)
    local part = Instance.new("Part")
    part.Name = payload.name
    part.Anchored = payload.anchored
    part.Position = Vector3.new(payload.position.x, payload.position.y, payload.position.z)
    part.Size = Vector3.new(payload.size.x, payload.size.y, payload.size.z)
    part.Parent = workspace
    return true, { output = "created:" .. part:GetFullName(), data = { fullName = part:GetFullName(), anchored = part.Anchored } }
end

local function handleGetScriptSource(payload)
    local inst, err = resolvePath(payload.path)
    if not inst then return false, { error = err } end
    if not inst:IsA("LuaSourceContainer") then return false, { error = inst:GetFullName() .. " is not a script." } end
    return true, { output = inst.Source, data = { fullName = inst:GetFullName(), className = inst.ClassName, lineCount = #inst.Source:split("\n") } }
end

local function handleSetScriptSource(payload)
    local inst, err = resolvePath(payload.path)
    if not inst then return false, { error = err } end
    if not inst:IsA("LuaSourceContainer") then return false, { error = inst:GetFullName() .. " is not a script." } end
    inst.Source = payload.source
    return true, { output = "updated:" .. inst:GetFullName(), data = { fullName = inst:GetFullName(), lineCount = #payload.source:split("\n") } }
end

local function handleEditScriptLines(payload)
    local inst, err = resolvePath(payload.path)
    if not inst then return false, { error = err } end
    if not inst:IsA("LuaSourceContainer") then return false, { error = inst:GetFullName() .. " is not a script." } end
    local src = inst.Source
    local startIdx = string.find(src, payload.oldString, 1, true)
    if not startIdx then return false, { error = "oldString not found in " .. inst:GetFullName() } end
    inst.Source = string.sub(src, 1, startIdx - 1) .. payload.newString .. string.sub(src, startIdx + #payload.oldString)
    return true, { output = "edited:" .. inst:GetFullName() }
end

local function handleGrepScripts(payload)
    local root, err = resolvePath(payload.rootPath or "game")
    if not root then return false, { error = err } end
    local results = {}
    for _, desc in ipairs(root:GetDescendants()) do
        if desc:IsA("LuaSourceContainer") then
            local src = desc.Source
            local pattern = payload.pattern
            if not payload.caseSensitive then src = string.lower(src); pattern = string.lower(pattern) end
            if string.find(src, pattern, 1, true) then
                table.insert(results, { path = desc:GetFullName(), className = desc.ClassName })
            end
        end
    end
    return true, { output = tostring(#results) .. " matches", data = results }
end

local function handleCreateInstance(payload)
    local parent, err = resolvePath(payload.parentPath)
    if not parent then return false, { error = err } end
    local inst = Instance.new(payload.className)
    if payload.name then inst.Name = payload.name end
    if payload.properties then
        for k, v in pairs(payload.properties) do pcall(function() inst[k] = v end) end
    end
    inst.Parent = parent
    return true, { output = "created:" .. inst:GetFullName(), data = { fullName = inst:GetFullName(), className = inst.ClassName } }
end

local function handleDeleteInstance(payload)
    local inst, err = resolvePath(payload.path)
    if not inst then return false, { error = err } end
    local name = inst:GetFullName()
    inst:Destroy()
    return true, { output = "deleted:" .. name }
end

local function handleSetProperty(payload)
    local inst, err = resolvePath(payload.path)
    if not inst then return false, { error = err } end
    local ok2, setErr = pcall(function() inst[payload.property] = payload.value end)
    if not ok2 then return false, { error = tostring(setErr) } end
    return true, { output = "set:" .. inst:GetFullName() .. "." .. payload.property }
end

local function handleCloneInstance(payload)
    local inst, err = resolvePath(payload.path)
    if not inst then return false, { error = err } end
    local parent, perr = resolvePath(payload.parentPath)
    if not parent then return false, { error = perr } end
    local clone = inst:Clone()
    clone.Parent = parent
    return true, { output = "cloned:" .. clone:GetFullName(), data = { fullName = clone:GetFullName() } }
end

local function handleReparentInstance(payload)
    local inst, err = resolvePath(payload.path)
    if not inst then return false, { error = err } end
    local parent, perr = resolvePath(payload.newParentPath)
    if not parent then return false, { error = perr } end
    inst.Parent = parent
    return true, { output = "reparented:" .. inst:GetFullName() }
end

local function handleGetInstanceProperties(payload)
    local inst, err = resolvePath(payload.path)
    if not inst then return false, { error = err } end
    local props = { Name = inst.Name, ClassName = inst.ClassName, FullName = inst:GetFullName() }
    if inst:IsA("BasePart") then
        props.Position = { x = inst.Position.X, y = inst.Position.Y, z = inst.Position.Z }
        props.Size = { x = inst.Size.X, y = inst.Size.Y, z = inst.Size.Z }
        props.Anchored = inst.Anchored
        props.Transparency = inst.Transparency
    end
    return true, { output = inst:GetFullName(), data = props }
end

local function handleGetInstanceChildren(payload)
    local inst, err = resolvePath(payload.path)
    if not inst then return false, { error = err } end
    local children = {}
    for _, child in ipairs(inst:GetChildren()) do
        table.insert(children, { name = child.Name, className = child.ClassName })
    end
    return true, { output = tostring(#children) .. " children", data = children }
end

local function handleSearchInstances(payload)
    local root, err = resolvePath(payload.rootPath or "game")
    if not root then return false, { error = err } end
    local results = {}
    for _, desc in ipairs(root:GetDescendants()) do
        local match = true
        if payload.className and desc.ClassName ~= payload.className then match = false end
        if payload.namePattern and not string.find(desc.Name, payload.namePattern, 1, true) then match = false end
        if match then table.insert(results, { path = desc:GetFullName(), className = desc.ClassName, name = desc.Name }) end
        if #results >= (payload.maxResults or 50) then break end
    end
    return true, { output = tostring(#results) .. " found", data = results }
end

local function handleGetFileTree(payload)
    local root, err = resolvePath(payload.rootPath or "game")
    if not root then return false, { error = err } end
    local function buildTree(node, depth, maxDepth)
        local entry = { name = node.Name, className = node.ClassName }
        if depth < maxDepth then
            local children = {}
            for _, child in ipairs(node:GetChildren()) do table.insert(children, buildTree(child, depth + 1, maxDepth)) end
            if #children > 0 then entry.children = children end
        end
        return entry
    end
    local tree = buildTree(root, 0, payload.maxDepth or 3)
    return true, { output = root:GetFullName(), data = tree }
end

local function handleSetGravity(payload)
    workspace.Gravity = Vector3.new(payload.x or 0, payload.y or -196.2, payload.z or 0)
    return true, { output = "gravity:" .. tostring(workspace.Gravity), data = { x = workspace.Gravity.X, y = workspace.Gravity.Y, z = workspace.Gravity.Z } }
end

local function handleSetPhysics(payload)
    local inst, err = resolvePath(payload.path)
    if not inst then return false, { error = err } end
    if not inst:IsA("BasePart") then return false, { error = inst:GetFullName() .. " is not a BasePart." } end
    inst.Anchored = payload.anchored
    if not payload.anchored and payload.velocity then
        inst.AssemblyLinearVelocity = Vector3.new(payload.velocity.x, payload.velocity.y, payload.velocity.z)
    end
    if not payload.anchored and payload.angularVelocity then
        inst.AssemblyAngularVelocity = Vector3.new(payload.angularVelocity.x, payload.angularVelocity.y, payload.angularVelocity.z)
    end
    return true, { output = "physics:" .. inst:GetFullName(), data = { anchored = inst.Anchored, fullName = inst:GetFullName() } }
end

local function handleAddConstraint(payload)
    local parent, err = resolvePath(payload.parentPath)
    if not parent then return false, { error = err } end
    local constraint = Instance.new(payload.constraintType)
    if payload.attachment0Path and payload.attachment0Path ~= "" then
        local a0, a0err = resolvePath(payload.attachment0Path)
        if not a0 then return false, { error = "Attachment0: " .. (a0err or "not found") } end
        constraint.Attachment0 = a0
    end
    if payload.attachment1Path and payload.attachment1Path ~= "" then
        local a1, a1err = resolvePath(payload.attachment1Path)
        if not a1 then return false, { error = "Attachment1: " .. (a1err or "not found") } end
        constraint.Attachment1 = a1
    end
    if payload.properties then
        for k, v in pairs(payload.properties) do pcall(function() constraint[k] = v end) end
    end
    constraint.Parent = parent
    return true, { output = "constraint:" .. constraint:GetFullName(), data = { className = constraint.ClassName, fullName = constraint:GetFullName() } }
end

local function handleRaycast(payload)
    local origin = Vector3.new(payload.origin.x, payload.origin.y, payload.origin.z)
    local dir = Vector3.new(payload.direction.x, payload.direction.y, payload.direction.z)
    local direction = dir.Unit * (payload.maxDistance or 1000)
    local params = RaycastParams.new()
    params.FilterType = Enum.RaycastFilterType[payload.filterType or "Exclude"]
    if payload.filterPaths and #payload.filterPaths > 0 then
        local list = {}
        for _, p in ipairs(payload.filterPaths) do
            local inst = resolvePath(p)
            if inst then table.insert(list, inst) end
        end
        params.FilterDescendantsInstances = list
    end
    local result = workspace:Raycast(origin, direction, params)
    if not result then return true, { output = "no_hit", data = { hit = false } } end
    return true, { output = "hit:" .. result.Instance:GetFullName(), data = {
        hit = true, instancePath = result.Instance:GetFullName(),
        position = { x = result.Position.X, y = result.Position.Y, z = result.Position.Z },
        normal = { x = result.Normal.X, y = result.Normal.Y, z = result.Normal.Z },
        distance = result.Distance, material = result.Material.Name,
    } }
end

local function handleSimulatePhysics(payload)
    local inst, err = resolvePath(payload.path)
    if not inst then return false, { error = err } end
    if not inst:IsA("BasePart") then return false, { error = inst:GetFullName() .. " is not a BasePart." } end
    if payload.impulse then
        local v = Vector3.new(payload.impulse.x, payload.impulse.y, payload.impulse.z)
        if payload.atPosition then
            inst:ApplyImpulseAtPosition(v, Vector3.new(payload.atPosition.x, payload.atPosition.y, payload.atPosition.z))
        else
            inst:ApplyImpulse(v)
        end
    end
    if payload.torque then
        inst:ApplyAngularImpulse(Vector3.new(payload.torque.x, payload.torque.y, payload.torque.z))
    end
    if payload.force then
        local att = Instance.new("Attachment"); att.Parent = inst
        local vf = Instance.new("VectorForce")
        vf.Force = Vector3.new(payload.force.x, payload.force.y, payload.force.z)
        vf.Attachment0 = att; vf.RelativeTo = Enum.ActuatorRelativeTo.World; vf.Parent = inst
    end
    return true, { output = "simulated:" .. inst:GetFullName() }
end

local COMMAND_HANDLERS = {
    run_code = handleRunCode,
    create_workspace_part = handleCreateWorkspacePart,
    get_script_source = handleGetScriptSource,
    set_script_source = handleSetScriptSource,
    edit_script_lines = handleEditScriptLines,
    grep_scripts = handleGrepScripts,
    create_instance = handleCreateInstance,
    delete_instance = handleDeleteInstance,
    set_property = handleSetProperty,
    clone_instance = handleCloneInstance,
    reparent_instance = handleReparentInstance,
    get_instance_properties = handleGetInstanceProperties,
    get_instance_children = handleGetInstanceChildren,
    search_instances = handleSearchInstances,
    get_file_tree = handleGetFileTree,
    set_gravity = handleSetGravity,
    set_physics = handleSetPhysics,
    add_constraint = handleAddConstraint,
    raycast = handleRaycast,
    simulate_physics = handleSimulatePhysics,
}

local function handleCommand(command)
    local handler = COMMAND_HANDLERS[command.kind]
    if handler then return handler(command.payload) end
    return false, { error = "Unsupported command kind: " .. tostring(command.kind) }
end

local function runtimeLoop()
    print("[MCP] Connecting to bridge at " .. CONFIG.baseUrl .. "...")
    local hs = postHandshake()
    if hs then
        print("[MCP] Handshake OK — runtime connected")
    else
        warn("[MCP] Handshake failed — is the MCP server running? (npm run dev)")
        return
    end

    running = true
    while running do
        local nextCommand = request("GET", CONFIG.baseUrl .. "/runtime/commands/next?timeoutMs=" .. tostring(CONFIG.commandTimeoutMs))
        if nextCommand and nextCommand.command then
            local ok, payload = handleCommand(nextCommand.command)
            submitResult(nextCommand.command.id, ok and "ok" or "error", payload)
        end
        postHandshake()
        task.wait(0.2)
    end
end

if plugin then
    local toolbar = plugin:CreateToolbar("Gamedev All-in-One MCP")
    local button = toolbar:CreateButton("Runtime", "Start/Stop MCP Runtime", "rbxassetid://4458901886")

    button.Click:Connect(function()
        if running then
            running = false
            print("[MCP] Runtime stopped")
        else
            task.spawn(runtimeLoop)
        end
    end)

    task.spawn(runtimeLoop)
    print("[MCP] Gamedev All-in-One plugin loaded — auto-connecting to bridge...")
else
    warn("[MCP] Not running as a plugin. Place this file in your Roblox Studio Plugins folder.")
end
