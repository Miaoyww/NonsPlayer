using System.Collections.ObjectModel;
using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Newtonsoft.Json.Linq;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Framework;
using NonsPlayer.Framework.Api;

namespace NonsPlayer.ViewModels;

public class HomeViewModel : ObservableRecipient, INotifyPropertyChanged
{
    public INavigationService NavigationService
    {
        get;
    }

    public ObservableCollection<PlaylistItem> Playlists = new();

    public HomeViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
    }

    public async void HomeLoad(object sender, RoutedEventArgs e)
    {
        var response = await Apis.Playlist.Personalized(Nons.Instance, 20);
        if ((int)response["code"] == 200)
        {
            var playlists = (JArray)response["result"];
            foreach (JObject item in playlists)
            {
                Playlists.Add(new PlaylistItem { PlayList = item });
            }
        }
    }
}