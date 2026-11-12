local RuntimeModule = require(script.Parent.init)

local PluginBootstrap = {}

local function createToolbarButton(pluginInstance)
    local toolbar = pluginInstance:CreateToolbar("GAMEDEV_ALL_IN_ONE")
    local button = toolbar:CreateButton(
        "Runtime",
        "Start the GAMEDEV_ALL_IN_ONE runtime loop",
        ""
    )

    button.Click:Connect(function()
        task.spawn(function()
            RuntimeModule.start()
        end)
    end)

    return button
end

function PluginBootstrap.start(pluginInstance)
    createToolbarButton(pluginInstance)

    task.spawn(function()
        RuntimeModule.start()
    end)
end

return PluginBootstrap
