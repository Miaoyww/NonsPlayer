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

    public void CardHide(object sender, PointerRoutedEventArgs e) => AnimationsHelper.CardHide(sender, e);

    public void CardShow(object sender, PointerRoutedEventArgs e) => AnimationsHelper.CardShow(sender, e);
    private Border getCard(string title, string id, string picUrl)
    {
        var bitmap = new BitmapImage(new Uri(picUrl));
        ImageBrush imageBrush = new();
        imageBrush.ImageSource = bitmap;
        var borderParent = new Border()
        {
            Margin = new Thickness(5, 10, 10, 10),
            Tag = id,
            CornerRadius = new CornerRadius(10),
            Background = new SolidColorBrush(Color.FromArgb(70, 190, 190, 190))
        };
        var grid = new Grid()
        {
            VerticalAlignment = VerticalAlignment.Center,
            HorizontalAlignment = HorizontalAlignment.Center
        };
        var borderCover = new Border()
        {
            Width = 230,
            Height = 230,
            Margin = new Thickness(8),
            CornerRadius = new CornerRadius(10),
            Background = imageBrush,
            VerticalAlignment = VerticalAlignment.Top,
            HorizontalAlignment = HorizontalAlignment.Center
        };
        var tblockTitle = new TextBlock()
        {
            Text = title,
            TextAlignment = TextAlignment.Left,
            Margin = new Thickness(0, 240, 0, 0),
            FontWeight = FontWeights.Black,
            FontSize = 18,
            Width = 224,
            Height = 58,
            TextWrapping = TextWrapping.WrapWholeWords,
            Padding = new Thickness(0, 0, 0, 0)
        };
        grid.Children.Add(borderCover);
        grid.Children.Add(tblockTitle);
        borderParent.PointerPressed += OpenMusicListDetail;
        borderParent.PointerEntered += AnimationsHelper.CardShow;
        borderParent.PointerExited += AnimationsHelper.CardHide;
        borderParent.Child = grid;
        return borderParent;
    }
    public async void HomeLoad(object sender, RoutedEventArgs e)
    {
        var response = await Api.Playlist.Personalized(ResEntry.nons, 20);
        if ((int)response["code"] == 200)
        {
            var playlists = (JArray)response["result"];
            foreach (JObject item in playlists)
            {
                await applyPanel(item, (HomePage)sender);
            }
        }
    }

    public async Task applyPanel(JObject item, HomePage p)
    {
        ;
        var picUrl = item["picUrl"].ToString() + "?param=200y200";
        var playlistView = getCard(
        (string)item["name"], item["id"].ToString(), picUrl);
        p.Panel_MusicList.Children.Add(playlistView);
    }

    private void OpenMusicListDetail(object sender, PointerRoutedEventArgs e)
    {
        Tools.OpenMusicListDetail(long.Parse(((Border)sender).Tag.ToString()!), NavigationService);
    }
}