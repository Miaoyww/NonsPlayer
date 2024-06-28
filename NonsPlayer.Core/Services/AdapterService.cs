using System.Reflection;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Services;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;

namespace NonsPlayer.Core.Services;

public class AdapterService
{
    private Dictionary<string, IAdapter> _adapters = new();
    private List<string> _disabledAdapters = new();
    public static AdapterService Instance { get; } = new();


    public void Init()
    {
        _disabledAdapters = ConfigManager.Instance.GetConfig("disabled_adapters").Get().Split(";;").ToList();
    }

    public void LoadAdapters(string directory)
    {
        string[] dllFiles = Directory.GetFiles(directory, "*.dll");

        foreach (var file in dllFiles)
        {
            var (name, assembly) = LoadSingleAdapter(file);
            _adapters.Add(name, assembly);
        }
    }

    public bool DisableAdapter(string platformName)
    {
        if (!_adapters.ContainsKey(platformName)) return false;
        if (_disabledAdapters.Contains(platformName)) return false;
        _disabledAdapters.Add(platformName);
        ConfigManager.Instance.GetConfig("disabledAdapters").Set(string.Join(";;", _disabledAdapters));
        return true;
    }

    public bool DisableAdapter(AdapterMetadata metadata)
    {
        return DisableAdapter(metadata.Platform);
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

    public IAdapter? GetAdapter(string platformName)
    {
        return _adapters.GetValueOrDefault(platformName);
    }

    public IAdapter[] GetLoadedAdapters()
    {
        return _adapters.Values.ToArray();
    }
}