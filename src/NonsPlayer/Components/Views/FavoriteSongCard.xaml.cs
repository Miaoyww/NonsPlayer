using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Animation;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class FavoriteSongCard : UserControl
{
    public FavoriteSongCard()
    {
        ViewModel = App.GetService<FavoriteSongCardViewModel>();
        InitializeComponent();
    }

    public FavoriteSongCardViewModel ViewModel { get; }

    public IMusic Music
    {
        set => ViewModel.Music = value;
    }
}