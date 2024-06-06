using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

public partial class HomeViewModel : ObservableRecipient
{
    public INavigationService NavigationService;

    [ObservableProperty] private ObservableCollection<Playlist>? recommendedPlaylist = new();

    public HomeViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
    }

    public async void HomePage_OnLoaded(object sender, RoutedEventArgs e)
    {
        var recommendedPlaylist = 
            await AdapterService.Instance.GetAdapter("ncm").Common.GetRecommendedPlaylistAsync(null, 6, NonsCore.Instance);
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            foreach (var item in recommendedPlaylist)
            {
                RecommendedPlaylist.Add(item);
            }
        });
    }
}