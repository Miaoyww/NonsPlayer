using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

public partial class PlaylistDetailViewModel : ObservableRecipient, INavigationAware, INotifyPropertyChanged
{
    private readonly double loadOffset = 500; // 到达底部多少距离时加载
    private readonly int musicItemGroupCount = 30; // 一次加载的MusicItem数量
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private string createTime;
    [ObservableProperty] private string creator;
    [ObservableProperty] private long currentId;
    private int currentItemGroupIndex;
    [ObservableProperty] private string description;
    [ObservableProperty] private bool isLiked;
    public ObservableCollection<MusicItem> MusicItems = new();
    [ObservableProperty] private string musicsCount;
    [ObservableProperty] private string name;
    [ObservableProperty] private Playlist playListObject;

    public void OnNavigatedFrom()
    {
    }

    public async void OnNavigatedTo(object parameter)
    {
        CurrentId = (long) parameter;
        UserPlaylistService.Instance.PlaylistUpdated += OnPlaylistUpdated;
        if (CurrentId.ToString() == FavoritePlaylistService.Instance.FavoritePlaylistId)
            if (FavoritePlaylistService.Instance.IsLikeSongsChanged)
            {
                await CacheHelper.UpdatePlaylistAsync(CurrentId + "_playlist", CurrentId.ToString());
                FavoritePlaylistService.Instance.IsLikeSongsChanged = false;
            }

        PlayListObject = await CacheHelper.GetPlaylistAsync(CurrentId + "_playlist", CurrentId.ToString());
        if (PlayListObject.IsCardMode)
        {
            var elapsed = await Tools.MeasureExecutionTimeAsync(PlayListObject.LoadAsync(PlayListObject.Id))
                .ConfigureAwait(false);
            Debug.WriteLine($"获取歌单Api耗时{elapsed.TotalMilliseconds}ms");
            PlayListObject.IsCardMode = false;
        }

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
            MusicsCount = PlayListObject.MusicsCount + "Tracks";
            Cover = CacheHelper.GetImageBrush(PlayListObject.CacheAvatarId, PlayListObject.AvatarUrl);
            IsLiked = UserPlaylistService.Instance.IsLiked(CurrentId);
        });
    }

    private async Task InitMusicsAsync()
    {
        if (PlayListObject.Musics == null)
        {
            PlayListObject.InitTracksAsync();
        }

        for (var i = 0; i < PlayListObject.Musics.Count; i++)
        {
            var index = i;
            if (index < musicItemGroupCount)
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    MusicItems.Add(new MusicItem
                    {
                        Music = PlayListObject.Musics[index],
                        Index = (index + 1).ToString("D2")
                    });
                });
        }

        currentItemGroupIndex = musicItemGroupCount;
    }

    public async void OnScrollViewerViewChanged(object? sender, ScrollViewerViewChangedEventArgs e)
    {
        if (sender is ScrollViewer scrollViewer)
        {
            var offset = scrollViewer.VerticalOffset;

            var height = scrollViewer.ScrollableHeight;
            if (height - offset < loadOffset && currentItemGroupIndex < playListObject.MusicsCount - 1)
                await LoadMusicItemsByGroup();
        }
    }

    /// <summary>
    ///     用于分组加载MusicItem
    /// </summary>
    private async Task LoadMusicItemsByGroup()
    {
        for (var i = 0; i < musicItemGroupCount; i++)
        {
            var index = currentItemGroupIndex + i + 1;
            if (index < PlayListObject.MusicsCount)
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    MusicItems.Add(new MusicItem
                    {
                        Music = PlayListObject.Musics[index],
                        Index = (index + 1).ToString("D2")
                    });
                });
        }

        currentItemGroupIndex += musicItemGroupCount;
    }


    [RelayCommand]
    private async void PlayAll()
    {
        if (playListObject.Musics.Count != playListObject.MusicTrackIds.Length)
        {
            // +1到达最后一个歌曲
            await playListObject.InitTrackByIndexAsync(1000, playListObject.MusicTrackIds.Length + 1);
        }

        PlayQueue.Instance.AddMusicList(playListObject.Musics.ToArray());
    }

    [RelayCommand]
    public async void Like()
    {
        await UserPlaylistService.Instance.Like(CurrentId).ConfigureAwait(false);
    }

    private void OnPlaylistUpdated()
    {
        IsLiked = UserPlaylistService.Instance.IsLiked(CurrentId);
    }

    public void DoubleClick(object sender, DoubleTappedRoutedEventArgs e)
    {
        var listView = sender as ListView;
        if (listView.SelectedItem is MusicItem item)
        {
            if (PlayQueue.Instance.Count == 0)
            {
                //TODO: 设置是否将歌曲添加到播放队列
                PlayAll();
            }

            // PlayQueue.Instance.Play(item.Music);
        }
    }
}