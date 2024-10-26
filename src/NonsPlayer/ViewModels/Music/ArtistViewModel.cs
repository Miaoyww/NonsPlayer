using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml;
using NonsPlayer.Components.Models;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using System.Collections.ObjectModel;
using System.Xml.Linq;
using Windows.Media.Playlists;

namespace NonsPlayer.ViewModels;

public partial class ArtistViewModel : ObservableRecipient, INavigationAware
{
    [ObservableProperty] public IArtist currentArtist;

    [ObservableProperty] private ImageBrush cover;
    private int currentItemGroupIndex;
    [ObservableProperty] private bool isLiked;
    public ObservableCollection<MusicModel> MusicItems = new();
    [ObservableProperty] private string musicsCount;
    [ObservableProperty] private string name;
    [ObservableProperty] private List<IMusic> songs;

    public void OnNavigatedTo(object parameter)
    {
        CurrentArtist = (IArtist)parameter;
        Songs = CurrentArtist.Songs.ToList();
        Name = CurrentArtist.Name;
        MusicsCount = Songs.Count + " Tracks";
        Task.Run(Init);
    }

    private async Task Init()
    {
        InitMusicsAsync();

        if (CurrentArtist is LocalArtist)
        {
            ServiceHelper.DispatcherQueue.TryEnqueue(async () =>
            {
                Cover = await ImageHelpers.GetImageBrushAsyncFromBytes(((LocalMusic)Songs[0]).GetCover());
            });
        }
    }

    public void OnNavigatedFrom()
    {
    }

    private void InitMusicsAsync()
    {
        for (var i = 0; i < Songs.Count; i++)
        {
            var index = i;
            if (index < AppConfig.Instance.AppSettings.PlaylistTrackCount)
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    MusicItems.Add(new MusicModel { Music = Songs[index], Index = (index + 1).ToString("D2") });
                });
        }

        currentItemGroupIndex =
            AppConfig.Instance.AppSettings.PlaylistTrackCount;
    }

    public async void OnScrollViewerViewChanged(object? sender, ScrollViewerViewChangedEventArgs e)
    {
        if (sender is ScrollViewer scrollViewer)
        {
            var offset = scrollViewer.VerticalOffset;

            var height = scrollViewer.ScrollableHeight;
            if (height - offset <
                AppConfig.Instance.AppSettings.PlaylistTrackCount &&
                currentItemGroupIndex < Songs.Count - 1)
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
            var index = currentItemGroupIndex + i;
            if (index < Songs.Count)
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    MusicItems.Add(new MusicModel { Music = Songs[index], Index = (index + 1).ToString("D2") });
                });
        }

        currentItemGroupIndex += AppConfig.Instance.AppSettings.PlaylistTrackCount;
    }


    [RelayCommand]
    private async void PlayAll()
    {
        PlayQueue.Instance.Clear();
        PlayQueue.Instance.AddMusicList(Songs.ToArray(), true);
    }
}