using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Components.Models;
using NonsPlayer.Core.Models;
using NonsPlayer.Services;
using System.Collections.ObjectModel;


namespace NonsPlayer.ViewModels;

public partial class LocalMusicLibViewModel : ObservableObject
{
    public ObservableCollection<MusicModel> Models = new();

    private LocalService localService = App.GetService<LocalService>();

    public LocalMusicLibViewModel()
    {
        Refresh();
    }

    public void Refresh()
    {
        var index = 0;
        foreach (LocalMusic localMusic in localService.Songs)
        {
            Models.Add(new MusicModel() { Index = index.ToString("D2"), Music = localMusic, });
            index++;
        }
    }
}