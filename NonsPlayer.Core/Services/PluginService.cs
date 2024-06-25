using System.Reflection;
using NonsPlayer.Core.Contracts.Plugins;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Services;

public class PluginService
{
    private Dictionary<string, IPlugin> _plugins = new();
    public static PluginService Instance { get; } = new();

    public void LoadPlugins(string directory)
    {
       
    }

    public Tuple<string, IPlugin>? LoadSingleAdapter(string file)
    {
        return null;
    }

    public IPlugin GetAdapter(string platformName)
    {
        return null;
    }
}