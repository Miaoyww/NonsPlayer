using System.Reflection;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Services;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;

namespace NonsPlayer.Core.Services;

public class AdapterService
{
    public List<Assembly> Adapters { get; set; } = new List<Assembly>();
    public static AdapterService Instance { get; } = new AdapterService();

    public List<Assembly> LoadAdapters(string directory)
    {
        var adapters = new List<Assembly>();
        string[] dllFiles = Directory.GetFiles(directory, "*.dll");

        foreach (var file in dllFiles)
        {
            adapters.Add(LoadSingleAdapter(file));
        }

        return adapters;
    }

    public Assembly? LoadSingleAdapter(string file)
    {
        try
        {
            var assembly = Assembly.LoadFrom(file);
            return assembly.GetTypes().Any(t => typeof(IAdapter).IsAssignableFrom(t)) ? assembly : null;
        }
        catch (Exception ex)
        {
            ExceptionService.Instance.Throw(ex, $"Error loading plugin from '{file}': {ex.Message}");
            return null;
        }
    }
}