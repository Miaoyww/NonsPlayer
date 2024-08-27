using ABI.Windows.Devices.Midi;
using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using System.Collections.ObjectModel;
using static NonsPlayer.Core.Services.ControlFactory;


namespace NonsPlayer.ViewModels;

public partial class LocalMusicLibViewModel : ObservableObject, INavigationAware
{
    public ObservableCollection<MusicModel> SongModels = new();

    private LocalService localService = App.GetService<LocalService>();

    public LocalMusicLibViewModel()
    {
        Refresh();
    }

    public void Refresh()
    {
        var index = 0;
        foreach (LocalMusic song in localService.Songs)
        {
            SongModels.Add(new MusicModel() { Index = index.ToString("D2"), Music = song, });
            index++;
        }
    }

    public void OnNavigatedTo(object parameter)
    {
    }

    public void OnNavigatedFrom()
    {
        SongModels.Clear();
    }
}