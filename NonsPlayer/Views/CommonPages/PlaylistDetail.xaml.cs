using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.CommonPages;

public sealed partial class PlaylistDetailPage : Page
{
    public PlaylistDetailViewModel ViewModel
    {
        get;
    }

    public PlaylistDetailPage()
    {
        ViewModel = App.GetService<PlaylistDetailViewModel>();
        InitializeComponent();

    }
    
}