using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Plugins;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Models.Github;
using NonsPlayer.Services;
using PluginModel = NonsPlayer.Models.Github.PluginModel;

namespace NonsPlayer.ViewModels;

public partial class AdapterManagerViewModel : ObservableRecipient, INavigationAware
{
    public ObservableCollection<PluginModel> PluginList = new();

    public List<PluginModel> Installed = new();
    public List<PluginModel> Plugins = new();
    public List<PluginModel> Adapters = new();
    public List<IAdapter> LocalAdapters = new();
    public ConfigManager ConfigManager => ConfigManager.Instance;

    [ObservableProperty] private string pluginCount;
    [ObservableProperty] private string updateTime;
    [ObservableProperty] private string adapterPath;

    public AdapterManagerViewModel()
    {
        AdapterPath = ConfigManager.Settings.AdapterPath;
    }

    public void OnNavigatedTo(object parameter)
    {
        Refresh();
    }

    public void Refresh()
    {
        LocalAdapters.Clear();
        Installed.Clear();
        foreach (var item in AdapterService.Instance.GetLoadedAdapters())
        {
            LocalAdapters.Add(item);
            foreach (PluginModel pluginModel in Adapters)
            {
                if (string.IsNullOrEmpty(pluginModel.Slug)) break;
                if (pluginModel.Slug.Equals(item.GetMetadata().Slug))
                {
                    Installed.Add(pluginModel);
                }
            }
        }
    }

    public void OnNavigatedFrom()
    {
    }
}