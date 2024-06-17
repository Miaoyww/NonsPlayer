using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Models;
using NonsPlayer.Services;

namespace NonsPlayer.ViewModels;

public partial class AdapterManagerViewModel : ObservableRecipient, INavigationAware
{
    public ObservableCollection<AdapterMetadata> Adapters { set; get; } = new();
    public ConfigManager ConfigManager { get; } = App.GetService<ConfigManager>();
    [ObservableProperty] private string adapterPath;

    public AdapterManagerViewModel()
    {
        AdapterPath = ConfigManager.GetConfig("adapterPath").Get();
    }

    public void OnNavigatedTo(object parameter)
    {
    }

    public void OnNavigatedFrom()
    {
    }
}