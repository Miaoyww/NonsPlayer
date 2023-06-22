using System.ComponentModel;
using System.Numerics;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Composition;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using Windows.UI;
using NonsApi;
using NonsPlayer.Components.Views;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Views.Pages;
using NonsPlayer.Framework.Api;
using NonsPlayer.Framework.Resources;

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