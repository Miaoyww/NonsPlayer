using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Newtonsoft.Json.Linq;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.Views;

public sealed partial class PlaylistCard : UserControl
{
    public PlaylistCardViewModel ViewModel
    {
        get;
    }

    public PlaylistCard()
    {
        ViewModel = App.GetService<PlaylistCardViewModel>();
        InitializeComponent();
    }

    public JObject PlaylistItem
    {
        set => ViewModel.Init(value);
    }

    private void CardShow(object sender, PointerRoutedEventArgs e) => AnimationsHelper.CardShow(sender, e);
    private void CardHide(object sender, PointerRoutedEventArgs e) => AnimationsHelper.CardHide(sender, e);
}