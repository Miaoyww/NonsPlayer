using System.Reflection;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Services;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;

namespace NonsPlayer.Core.Services;

public class AdapterService
{
    private Dictionary<string, IAdapter> _adapters = new();
    public static AdapterService Instance { get; } = new();

    public void LoadAdapters(string directory)
    {
        string[] dllFiles = Directory.GetFiles(directory, "*.dll");

        foreach (var file in dllFiles)
        {
            var (name, assembly) = LoadSingleAdapter(file);
            _adapters.Add(name, assembly);
        }
    }

    public Tuple<string, IAdapter>? LoadSingleAdapter(string file)
    {
        try
        {
            var assembly = Assembly.LoadFrom(file);
            IAdapter adapter;
            foreach (var item in assembly.GetTypes())
            {
                if (item.FullName.Contains("Adapters.Adapter"))
                {
                    adapter = (IAdapter)Activator.CreateInstance(item);
                    return new Tuple<string, IAdapter>(adapter.GetMetadata().Platform, adapter);
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

    public IAdapter GetAdapter(string platformName)
    {
        return _adapters.TryGetValue(platformName, out var adapter) ? adapter : null;
    }
}