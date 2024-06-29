using System.Reflection;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Services;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;

namespace NonsPlayer.Core.Services;

public class AdapterService
{
    private Dictionary<string, IAdapter> _adapters = new();
    private Dictionary<string, IAdapter> _disabledAdapters = new();
    private List<string> _disabledAdaptersStrings = new();
    public static AdapterService Instance { get; } = new();


    public void Init()
    {
        _disabledAdaptersStrings = ConfigManager.Instance.Settings.DisabledAdapters.Split(";;")
            .SkipWhile(x => x.Equals(string.Empty)).ToList();
        LoadAdapters(ConfigManager.Instance.Settings.AdapterPath);
    }

    public void LoadAdapters(string directory)
    {
        if (!Directory.Exists(directory)) Directory.CreateDirectory(directory);
        string[] dllFiles = Directory.GetFiles(directory, "*.dll");

        foreach (var file in dllFiles)
        {
            var (name, assembly) = LoadSingleAdapter(file);
            if (_disabledAdaptersStrings.Contains(assembly.GetMetadata().Platform))
            {
                _disabledAdapters.Add(name, assembly);
                continue;
            }

            _adapters.Add(name, assembly);
        }
    }

    public bool DisableAdapter(string platformName)
    {
        if (!_adapters.ContainsKey(platformName)) return false;
        if (_disabledAdaptersStrings.Contains(platformName)) return false;
        _disabledAdaptersStrings.Add(platformName);
        ConfigManager.Instance.Settings.DisabledAdapters = string.Join(";;", _disabledAdaptersStrings);
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

    public IAdapter[] GetAdaptersByType(ISubAdapterEnum type)
    {
        return _adapters.Values.Where(adapter => adapter.GetMetadata().Types.Contains(type)).ToArray();
    }

    public IAdapter[] GetLoadedAdapters()
    {
        return _adapters.Values.ToArray();
    }
}