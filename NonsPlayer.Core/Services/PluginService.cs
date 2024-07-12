using System.Reflection;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Plugins;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Services;

public class PluginService
{
    private Dictionary<string, IPlugin> _plugins = new();
    private Dictionary<string, IPlugin> _disabledPlugins = new();
    public static PluginService Instance { get; } = new();
    private List<string> _disabledPluginsStrings = new();

    public void LoadPlugins(string directory)
    {
        if (!Directory.Exists(directory)) Directory.CreateDirectory(directory);
        string[] dllFiles = Directory.GetFiles(directory, "*.dll");

        foreach (var file in dllFiles)
        {
            var (name, assembly) = LoadSinglePlugin(file);
            if (_disabledPluginsStrings.Contains(assembly.GetMetadata().Name))
            {
                _disabledPlugins.Add(name, assembly);
                continue;
            }

            _plugins.Add(name, assembly);
        }
    }

    public bool DisablePlugin(string platformName)
    {
        if (!_plugins.ContainsKey(platformName)) return false;
        if (_disabledPluginsStrings.Contains(platformName)) return false;
        _disabledPluginsStrings.Add(platformName);
        ConfigManager.Instance.Settings.DisabledAdapters = string.Join(";;", _disabledPluginsStrings);
        return true;
    }

    public bool DisablePlugin(AdapterMetadata metadata)
    {
        return DisablePlugin(metadata.Platform);
    }

    public Tuple<string, IPlugin>? LoadSinglePlugin(string file)
    {
        try
        {
            var assembly = Assembly.LoadFrom(file);
            IPlugin plugin;
            foreach (var item in assembly.GetTypes())
            {
                if (item.FullName.Contains("Adapters.Adapter"))
                {
                    plugin = (IPlugin)Activator.CreateInstance(item);
                    return new Tuple<string, IPlugin>(plugin.GetMetadata().Name, plugin);
                }
            }

            return null;
        }
        catch (Exception ex)
        {
            ExceptionService.Instance.Throw(ex, $"Error loading plugin from '{file}': {ex.Message}");
            return null;
        }
    }

}