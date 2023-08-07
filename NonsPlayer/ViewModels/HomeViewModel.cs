using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

public partial class HomeViewModel : ObservableRecipient
{
    public INavigationService NavigationService;

    [ObservableProperty] public ObservableCollection<Playlist>? recommendedPlaylist = new();

    public HomeViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
    }

    public async void HomePage_OnLoaded(object sender, RoutedEventArgs e)
    {
        var response = await Apis.Playlist.Personalized(Nons.Instance, 20).ConfigureAwait(false);
        if ((int) response["code"] != 200)
            //TODO: 处理此错误
            return;

        var playlists = (JArray) response["result"];
        var tasks = playlists.Select(item =>
            CacheHelper.GetPlaylistCardAsync(item["id"] + "_playlist", (JObject) item));
        var results = await Task.WhenAll(tasks);
        results.ToList().ForEach(item =>
        {
            ServiceHelper.DispatcherQueue.TryEnqueue(() => { RecommendedPlaylist.Add(item); });
        });
    }
}