using Microsoft.UI.Xaml.Controls;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.CommonPages;

public sealed partial class PlaylistDetailPage : Page
{
    public PlayListDetailViewModel ViewModel
    {
        get;
    }

    public PlaylistDetailPage()
    {
        ViewModel = App.GetService<PlayListDetailViewModel>();
        InitializeComponent();

    }
}