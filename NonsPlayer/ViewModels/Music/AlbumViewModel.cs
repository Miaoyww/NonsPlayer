using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;

namespace NonsPlayer.ViewModels;

public partial class AlbumViewModel : ObservableRecipient, INavigationAware
{
    [ObservableProperty] public IAlbum currentAlbum;

    public async void OnNavigatedTo(object parameter)
    {
        CurrentAlbum = (IAlbum)parameter;
    }

    public void OnNavigatedFrom()
    {
    }
}