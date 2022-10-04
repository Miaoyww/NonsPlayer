using System.Drawing;
using System.IO;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NcmApi;
using NcmPlayer.Contracts.Services;
using NcmPlayer.Resources;
using NcmPlayer.Views;
using Newtonsoft.Json.Linq;

namespace NcmPlayer.ViewModels;

public class HomeViewModel : ObservableRecipient
{
    public INavigationService NavigationService
    {
        get;
    }

    public HomeViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
    }

    private StackPanel getStackPanel(string title, string id, string picUrl)
    {
        BitmapImage bitmap = new BitmapImage(new Uri(picUrl));
        ImageBrush imageBrush = new();
        imageBrush.ImageSource = bitmap;


        var stackPanel = new StackPanel()
        {
            Margin = new Thickness(0, 0, 20, 0),
        };
        var borderCover = new Border()
        {
            Width = 200,
            Height = 200,
            Tag = id,
            CornerRadius = new CornerRadius(20),
            Background = imageBrush
        };
        borderCover.PointerPressed += OpenMusicListDetail;
        var tblockTitle = new TextBlock()
        {
            Text = title,
            TextAlignment = TextAlignment.Left,
            Margin = new Thickness(0, 15, 0, 0),
            FontWeight = FontWeights.Bold,
            FontSize = 20,
            Width = 200,
            Height = 94,
            TextWrapping = TextWrapping.WrapWholeWords,
            Padding = new Thickness(0, 0, 0, 0)
        };
        stackPanel.Children.Add(borderCover);
        stackPanel.Children.Add(tblockTitle);
        return stackPanel;
    }

    public async void HomeLoad(object sender, Microsoft.UI.Xaml.RoutedEventArgs e)
    {
        JObject response = Api.Playlist.Personalized(ResEntry.ncm, 20).Result;
        if ((int)response["code"] == 200)
        {
            JArray playlists = (JArray)response["result"];
            foreach (JObject item in playlists)
            {
                applyPanel(item, (HomePage)sender);
            }
        }
    }

    public async void applyPanel(JObject item, HomePage p)
    {
        // Stream playlistCover = HttpRequest.StreamHttpGet(item["picUrl"].ToString() + "?param=180y180").Result;
        string picUrl = item["picUrl"].ToString() + "?param=140y140";
        StackPanel playlistView = getStackPanel(
        (string)item["name"], item["id"].ToString(), picUrl);
        p.Panel_MusicList.Children.Add(playlistView);
    }

    private void OpenMusicListDetail(object sender, Microsoft.UI.Xaml.Input.PointerRoutedEventArgs e)
    {
        Tool.OpenMusicListDetail(long.Parse(((Border)sender).Tag.ToString()), NavigationService);
    }
}