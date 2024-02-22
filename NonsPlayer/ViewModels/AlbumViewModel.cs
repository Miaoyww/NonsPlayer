using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Models;

namespace NonsPlayer.ViewModels;

public partial class AlbumViewModel : ObservableRecipient, INavigationAware
{
    [ObservableProperty] public Album currentAlbum;

    public async void OnNavigatedTo(object parameter)
    {
        CurrentAlbum = (Album)parameter;
    }

    public void OnNavigatedFrom()
    {
    }
}