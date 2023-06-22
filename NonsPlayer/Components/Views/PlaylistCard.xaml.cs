using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Newtonsoft.Json.Linq;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.Views;

public sealed partial class PlaylistCard : Page
{
    public PlaylistCardViewModel ViewModel
    {
        get;
    }

    private readonly JObject playlistItem;

    public PlaylistCard(JObject item)
    {
        ViewModel = App.GetService<PlaylistCardViewModel>();
        InitializeComponent();
        playlistItem = item;
    }

    private void PlaylistCard_OnLoaded(object sender, RoutedEventArgs e)
    {
        ViewModel.Init(playlistItem);
    }

    private void CardShow(object sender, PointerRoutedEventArgs e) => AnimationsHelper.CardShow(sender, e);
    private void CardHide(object sender, PointerRoutedEventArgs e) => AnimationsHelper.CardHide(sender, e);
}