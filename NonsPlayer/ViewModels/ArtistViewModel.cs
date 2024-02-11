using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using System.Diagnostics;

namespace NonsPlayer.ViewModels;

public partial class ArtistViewModel : ObservableRecipient, INavigationAware
{
    [ObservableProperty] public Artist currentArtist;

    public async void OnNavigatedTo(object parameter)
    {
        CurrentArtist = (Artist)parameter;
    }

    public void OnNavigatedFrom()
    {
    }
}