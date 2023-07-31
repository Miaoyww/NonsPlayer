using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

public class HomeViewModel : ObservableRecipient, INavigationAware
{
    public INavigationService NavigationService
    {
        get;
    }

    public ObservableCollection<Playlist>? RecommendedPlaylist;

    public HomeViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
    }

    public async void OnNavigatedTo(object e)
    {
        if (RecommendedPlaylist == null)
        {
            RecommendedPlaylist = new();
            var response = await Apis.Playlist.Personalized(Nons.Instance, 20);
            if ((int)response["code"] != 200)
            {
                //TODO: 处理此错误
                return;
            }
            
            var playlists = (JArray)response["result"];
            var tasks = playlists.Select(item =>
                CacheHelper.GetPlaylistAsync(item["id"] + "_playlist", item["id"].ToString()));
            var results = await Task.WhenAll(tasks);
            results.ToList().ForEach(item => RecommendedPlaylist.Add(item));
        }
    }

    public void OnNavigatedFrom()
    {
        return;
    }
}