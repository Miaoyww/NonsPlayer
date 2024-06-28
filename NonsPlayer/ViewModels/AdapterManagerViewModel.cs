using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Plugins;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Services;

namespace NonsPlayer.ViewModels;

public partial class AdapterManagerViewModel : ObservableRecipient, INavigationAware
{
    public ObservableCollection<AdapterModel> InstalledAdapters { set; get; } = new();
    public ObservableCollection<PluginModel> InstalledPlugins { set; get; } = new();
    public ConfigManager ConfigManager => ConfigManager.Instance;
    private readonly VersionService _versionService = App.GetService<VersionService>();

    [ObservableProperty] private string versionDescription;
    [ObservableProperty] private string adapterPath;

    public AdapterManagerViewModel()
    {
        AdapterPath = ConfigManager.GetConfig("adapterPath").Get();
        VersionDescription = _versionService.CurrentVersionDescription;
    }

    public void OnNavigatedTo(object parameter)
    {
        Refresh();
    }

    public void Refresh()
    {
        var adapters = AdapterService.Instance.GetLoadedAdapters();
        for (int i = 0; i < adapters.Length; i++)
        {
            InstalledAdapters.Add(new AdapterModel()
            {
                Index = i + 1,
                Metadata = adapters[i].GetMetadata()
            });
        }

        // var plugins = PluginService.Instance.GetLoadedPlugins();
    }

    public void OnNavigatedFrom()
    {
    }
}