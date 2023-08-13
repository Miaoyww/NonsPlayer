using Microsoft.UI.Xaml.Controls;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views;

public sealed partial class PlaylistDetailPage : Page
{
    public PlaylistDetailPage()
    {
        ViewModel = App.GetService<PlaylistDetailViewModel>();
        InitializeComponent();
    }

    public PlaylistDetailViewModel ViewModel { get; }
}