using Microsoft.UI.Xaml.Controls;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views;

public sealed partial class ArtistPage : Page
{
    public ArtistPage()
    {
        ViewModel = App.GetService<ArtistViewModel>();
        InitializeComponent();
    }

    public ArtistViewModel ViewModel { get; }
}