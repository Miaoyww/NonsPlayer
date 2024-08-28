using ABI.Windows.Devices.Midi;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;
using NonsPlayer.Models;
using NonsPlayer.Services;
using System.Collections.ObjectModel;
using static NonsPlayer.Core.Services.ControlFactory;


namespace NonsPlayer.ViewModels;

public partial class LocalMusicLibViewModel : ObservableObject, INavigationAware
{
    public ObservableCollection<MusicModel> SongModels = new();
    public ObservableCollection<LocalArtistModel> ArtistModels = new();
    public ObservableCollection<LocalAlbumModel> AlbumModels = new();

    private LocalService localService = App.GetService<LocalService>();

    public LocalMusicLibViewModel()
    {
        Refresh();
    }

    public async void Refresh()
    {
        var index = 0;
        SongModels.Clear();
        foreach (LocalMusic song in localService.Songs)
        {
            index++;
            SongModels.Add(new MusicModel() { Index = index.ToString("D2"), Music = song, });
            if (!song.IsInit) song.Init();
            
            if (song.Artists != null)
            {
                foreach (LocalArtist artist in song.Artists)
                {
                    var existingArtist = localService.Artists.FirstOrDefault(a => a.Equals(artist));
                    if (existingArtist != null)
                    {
                        existingArtist.Songs.Add(song);
                    }
                    else
                    {
                        localService.Artists.Add(artist);
                    }
                }
            }

            if (song.Album != null)
            {
                var existingAlbum = localService.Albums.FirstOrDefault(a => a.Equals(song.Album));
                if (existingAlbum != null)
                {
                    existingAlbum.Songs.Add(song);
                }
                else
                {
                    ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                    {
                        localService.Albums.Add((LocalAlbum)song.Album);
                    });
                }
            }
        }

        var index2 = 0;
        ArtistModels.Clear();
        foreach (LocalArtist artist in localService.Artists)
        {
            index2++;
            ArtistModels.Add(new LocalArtistModel { Artist = artist, Index = index2.ToString("D2") });
        }

        var index3 = 0;
        AlbumModels.Clear();
        foreach (LocalAlbum album in localService.Albums)
        {
            index3++;
            AlbumModels.Add(new LocalAlbumModel { Album = album, Index = index3.ToString("D2") });
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