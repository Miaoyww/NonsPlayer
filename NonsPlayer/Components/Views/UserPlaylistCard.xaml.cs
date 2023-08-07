using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace NonsPlayer.Components.Views;

public sealed partial class UserPlaylistCard : UserControl
{
    public UserPlaylistCard()
    {
        ViewModel = App.GetService<UserPlaylistCardViewModel>();
        InitializeComponent();
    }

    public UserPlaylistCardViewModel ViewModel { get; }

    public Playlist PlaylistItem
    {
        set => ViewModel.Init(value);
    }

    private void CardShow(object sender, PointerRoutedEventArgs e)
    {
        AnimationsHelper.CardShow(sender, e);
    }

    private void CardHide(object sender, PointerRoutedEventArgs e)
    {
        AnimationsHelper.CardHide(sender, e);
    }
}