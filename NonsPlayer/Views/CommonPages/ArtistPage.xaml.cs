using Microsoft.UI.Xaml.Controls;

using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.CommonPages;

public sealed partial class ArtistPage : Page
{
    public ArtistViewModel ViewModel
    {
        get;
    }

    public ArtistPage()
    {
        ViewModel = App.GetService<ArtistViewModel>();
        InitializeComponent();
    }
}
