using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Models;
using NonsPlayer.Services;
using System.Collections.ObjectModel;


namespace NonsPlayer.ViewModels;

public partial class LocalMusicLibViewModel : ObservableObject, INavigationAware
{
    public ObservableCollection<MusicModel> Models = new();

    private LocalService localService = App.GetService<LocalService>();

    public LocalMusicLibViewModel()
    {
        Task.Run(InitMusics);
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

    public async Task InitMusics()
    {
        List<Task> tasks = new();
        foreach (MusicModel musicModel in Models)
        {
            if (!((LocalMusic)musicModel.Music).IsInit)
            {
                tasks.Add(Task.Run(() => { ((LocalMusic)musicModel.Music).Init(); }));
            }
        }

        await Task.WhenAll(tasks);
        Refresh();
    }

    public void OnNavigatedTo(object parameter)
    {
        
    }

    public void OnNavigatedFrom()
    {
        Models.Clear();
    }
}