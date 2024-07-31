using System.Diagnostics;
using System.Reflection;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Services;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;

namespace NonsPlayer.Core.Services;

public class AdapterService
{
    #region 事件注册

    public delegate void AdapterEventHandler(string param);

    public event AdapterEventHandler? AdapterLoadFailed;
    public event AdapterEventHandler? AdapterLoading;

    #endregion

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
            try
            {
                var (name, assembly) = LoadSingleAdapter(file);
                if (name == null) continue;
                if (assembly == null) continue;

                if (_disabledAdaptersStrings.Contains(assembly.GetMetadata().Platform))
                {
                    _disabledAdapters.Add(name, assembly);
                    continue;
                }

                _adapters.Add(name, assembly);
            }
            catch (NullReferenceException ex)
            {
            }
        }
    }

    public bool DisableAdapter(string name)
    {
        if (!_adapters.ContainsKey(name)) return false;
        if (_disabledAdaptersStrings.Contains(name)) return false;
        _disabledAdaptersStrings.Add(name);
        ConfigManager.Instance.Settings.DisabledAdapters = string.Join(";;", _disabledAdaptersStrings);
        return true;
    }

    public bool DisableAdapter(AdapterMetadata metadata)
    {
        return DisableAdapter(metadata.Platform);
    }

    public Tuple<string?, IAdapter?>? LoadSingleAdapter(string file)
    {
        try
        {
            AdapterLoading?.Invoke(file);
            var assembly = Assembly.LoadFrom(file);
            IAdapter adapter;
            foreach (var item in assembly.GetTypes())
            {
                if (item.FullName.Contains("Adapters.Adapter"))
                {
                    adapter = (IAdapter)Activator.CreateInstance(item);
                    return new Tuple<string, IAdapter>(adapter.GetMetadata().Name, adapter);
                }
            }

            AdapterLoadFailed?.Invoke(file);
            return new Tuple<string?, IAdapter?>(null, null);
        }
        catch (Exception ex)
        {
            AdapterLoadFailed?.Invoke(file);
            return new Tuple<string?, IAdapter?>(null, null);
        }
    }

    public IAdapter? GetAdapter(string name)
    {
        return _adapters.GetValueOrDefault(name);
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