using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using static NonsPlayer.Core.Services.ControlFactory;

namespace NonsPlayer.ViewModels;

public partial class HomeViewModel : ObservableRecipient
{
    public INavigationService NavigationService;

    [ObservableProperty] private ObservableCollection<IPlaylist>? recommendedPlaylist = new();

    public HomeViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
    }

    public async void HomePage_OnLoaded(object sender, RoutedEventArgs e)
    {
        var adapters = AdapterService.Instance.GetAdaptersByType(ISubAdapterEnum.Common);
        List<IPlaylist> recommendedPlaylist = new();
        foreach (var item in adapters)
        {
            recommendedPlaylist.AddRange(await item.Common.GetRecommendedPlaylistAsync(null, 6, NonsCore.Instance));
        }
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            foreach (var item in recommendedPlaylist)
            {
                RecommendedPlaylist.Add(item);
            }
        });
    }
}