using ABI.Windows.Devices.Midi;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Utils;
using NonsPlayer.Helpers;
using NonsPlayer.Models;
using NonsPlayer.Services;
using System.Collections.Concurrent;
using System.Collections.ObjectModel;
using static NonsPlayer.Core.Services.ControlFactory;


namespace NonsPlayer.ViewModels;

public partial class LocalMusicLibViewModel : ObservableObject, INavigationAware
{
    public ObservableCollection<MusicModel> SongModels = new();
    public ObservableCollection<LocalArtistModel> ArtistModels = new();
    public ObservableCollection<LocalAlbumModel> AlbumModels = new();
    private int taskCount = 8;
    private int completedTasks = 0;
    private LocalService localService = App.GetService<LocalService>();
    private int currentItemGroupIndex;

    public LocalMusicLibViewModel()
    {
        Refresh();
    }

    public async void Refresh()
    {
        var index = 0;
        SongModels.Clear();
        await LoadMusicItemsByGroup();
        foreach (LocalMusic song in localService.Songs)
        {
            index++;
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

    public async void OnScrollViewerViewChanged(object? sender, ScrollViewerViewChangedEventArgs e)
    {
        if (sender is ScrollViewer scrollViewer)
        {
            var offset = scrollViewer.VerticalOffset;
        
            var height = scrollViewer.ScrollableHeight;
            if (height - offset <
                AppConfig.Instance.AppSettings.PlaylistTrackCount &&
                currentItemGroupIndex < localService.Songs.Count - 1)
                await LoadMusicItemsByGroup();
        }
    }

    /// <summary>
    ///     用于分组加载MusicItem
    /// </summary>
    private async Task LoadMusicItemsByGroup()
    {
        for (var i = 0; i < AppConfig.Instance.AppSettings.PlaylistTrackCount; i++)
        {
            int currentIndex = currentItemGroupIndex + i;
            if (currentIndex >= localService.Songs.Count) break; // 确保索引有效
            GlobalThreadPool.Instance.Enqueue(() =>
            {
                var music = localService.Songs[currentIndex];
                if (!music.IsInit)
                {
                    music.Init();
                    music.Cover = LocalUtils.CompressAndConvertToByteArray(music.GetCover(), 80, 80);
                }

                var model = new MusicModel { Music = music, Index = (currentIndex + 1).ToString("D2") };
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    SongModels.Add(model);
                });
            });
        }

        currentItemGroupIndex += AppConfig.Instance.AppSettings.PlaylistTrackCount;
    }

    public void OnNavigatedTo(object parameter)
    {
    }

    public void OnNavigatedFrom()
    {
        SongModels.Clear();
    }
}