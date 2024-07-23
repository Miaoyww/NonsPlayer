using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

public partial class PlaylistDetailViewModel : ObservableRecipient, INavigationAware, INotifyPropertyChanged
{
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private string createTime;
    [ObservableProperty] private string creator;
    [ObservableProperty] private string currentId;
    private int currentItemGroupIndex;
    [ObservableProperty] private string description;
    [ObservableProperty] private bool isLiked;
    public ObservableCollection<MusicModel> MusicItems = new();
    [ObservableProperty] private string musicsCount;
    [ObservableProperty] private string name;
    [ObservableProperty] private IPlaylist playListObject;

    public void OnNavigatedFrom()
    {
    }

    public async void OnNavigatedTo(object parameter)
    {
        PlayListObject = (IPlaylist)parameter;
        CurrentId = PlayListObject.Id;
        if (!PlayListObject.IsInitialized) await PlayListObject.InitializePlaylist();
        LoadPlaylistDetail();
        await InitMusicsAsync().ConfigureAwait(false);
    }

    private void LoadPlaylistDetail()
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            Name = PlayListObject.Name;
            Creator = "made by " + PlayListObject.Creator;
            CreateTime = $"· {PlayListObject.CreateTime.ToString().Split(" ")[0]}";
            Description = PlayListObject.Description;
            MusicsCount = PlayListObject.MusicsCount + " Tracks";
            Cover = CacheHelper.GetImageBrush(PlayListObject.CacheAvatarId, PlayListObject.AvatarUrl);
            // IsLiked = UserPlaylistService.Instance.IsLiked(CurrentId);
        });
    }

    private async Task InitMusicsAsync()
    {
        if (PlayListObject.Musics == null) await PlayListObject.InitializeMusics();

        for (var i = 0; i < PlayListObject.Musics.Count; i++)
        {
            var index = i;
            if (index < AppConfig.PlaylistTrackShowCount)
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    MusicItems.Add(new MusicModel
                    {
                        Music = PlayListObject.Musics[index],
                        Index = (index + 1).ToString("D2")
                    });
                });
        }

        currentItemGroupIndex =
            AppConfig.PlaylistTrackShowCount;
    }

    public async void OnScrollViewerViewChanged(object? sender, ScrollViewerViewChangedEventArgs e)
    {
        if (sender is ScrollViewer scrollViewer)
        {
            var offset = scrollViewer.VerticalOffset;

            var height = scrollViewer.ScrollableHeight;
            if (height - offset <
                AppConfig.PlaylistTrackShowCount &&
                currentItemGroupIndex < playListObject.MusicsCount - 1)
                await LoadMusicItemsByGroup();
        }
    }

    /// <summary>
    ///     用于分组加载MusicItem
    /// </summary>
    private async Task LoadMusicItemsByGroup()
    {
        for (var i = 0; i < AppConfig.PlaylistTrackShowCount; i++)
        {
            var index = currentItemGroupIndex + i;
            if (index < PlayListObject.MusicsCount)
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    MusicItems.Add(new MusicModel
                    {
                        Music = PlayListObject.Musics[index],
                        Index = (index + 1).ToString("D2")
                    });
                });
        }

        currentItemGroupIndex += AppConfig.PlaylistTrackShowCount;
    }


    [RelayCommand]
    private async void PlayAll()
    {
        if (playListObject.Musics.Count != playListObject.MusicTrackIds.Length)
            // +1到达最后一个歌曲
            // await playListObject.InitTrackByIndexAsync(1000, playListObject.MusicTrackIds.Length + 1);

        PlayQueue.Instance.Clear();
        PlayQueue.Instance.AddMusicList(playListObject.Musics.ToArray(), true);
    }

    [RelayCommand]
    public async void Like()
    {
        // await UserPlaylistService.Instance.Like(CurrentId).ConfigureAwait(false);
    }

    private void OnPlaylistUpdated()
    {
        // IsLiked = UserPlaylistService.Instance.IsLiked(CurrentId);
    }
}