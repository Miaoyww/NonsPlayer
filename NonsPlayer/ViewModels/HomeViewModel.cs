using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Newtonsoft.Json.Linq;
using NonsApi;
using NonsPlayer.Components.Views;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Framework.Resources;
using NonsPlayer.Views.Pages;

namespace NonsPlayer.ViewModels;

public class HomeViewModel : ObservableRecipient, INotifyPropertyChanged
{
    public INavigationService NavigationService
    {
        get;
    }

    public HomeViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
    }

    public async void HomeLoad(object sender, RoutedEventArgs e)
    {
        var response = await Api.Playlist.Personalized(ResEntry.nons, 20);
        if ((int)response["code"] == 200)
        {
            var playlists = (JArray)response["result"];
            foreach (JObject item in playlists)
            {
                ApplyPanelAsync(item, (HomePage)sender);
            }
        }
    }

    private static void ApplyPanelAsync(JObject item, HomePage p)
    {
        p.Panel_MusicList.Children.Add(new PlaylistCard(item));
    }
}