using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;

namespace NonsPlayer.ViewModels;

public partial class ArtistViewModel : ObservableRecipient, INavigationAware
{
    [ObservableProperty] public IArtist currentArtist;

    public async void OnNavigatedTo(object parameter)
    {
        CurrentArtist = (IArtist)parameter;
    }

    public void OnNavigatedFrom()
    {
    }
}