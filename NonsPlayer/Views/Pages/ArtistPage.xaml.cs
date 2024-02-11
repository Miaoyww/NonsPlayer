using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Core.Models;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class ArtistPage : Page
{
    public ArtistPage()
    {
        ViewModel = App.GetService<ArtistViewModel>();
        InitializeComponent();
    }
    
    public ArtistViewModel ViewModel { get; }
    
}