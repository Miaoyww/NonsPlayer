using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Plugins;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Services;

namespace NonsPlayer.ViewModels;

public partial class AdapterManagerViewModel : ObservableRecipient, INavigationAware
{
    public ObservableCollection<AdapterMetadata> InstalledAdapters { set; get; } = new();
    public ObservableCollection<PluginMetadata> InstalledPlugins { set; get; } = new();
    public ConfigManager ConfigManager { get; } = App.GetService<ConfigManager>();
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
        foreach (var item in AdapterService.Instance.GetLoadedAdapters())
        {
            InstalledAdapters.Add(item.GetMetadata());
        }
    }

    public void OnNavigatedFrom()
    {
    }
}