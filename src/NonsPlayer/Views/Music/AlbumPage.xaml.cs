using Microsoft.UI.Xaml.Controls;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class AlbumPage : Page
{
    public AlbumPage()
    {
        ViewModel = App.GetService<AlbumViewModel>();
        InitializeComponent();
    }

    public AlbumViewModel ViewModel { get; }
}