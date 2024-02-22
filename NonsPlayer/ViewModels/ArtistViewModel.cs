using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Models;

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