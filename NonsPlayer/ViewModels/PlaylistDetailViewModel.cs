using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Cache;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;

namespace NonsPlayer.ViewModels;

public partial class PlaylistDetailViewModel : ObservableRecipient, INavigationAware, INotifyPropertyChanged
{
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private Playlist playListObject;
    [ObservableProperty] private string createTime;
    [ObservableProperty] private string creator;
    [ObservableProperty] private long currentId;
    [ObservableProperty] private string description;
    [ObservableProperty] private string musicsCount;
    [ObservableProperty] private string name;
    [ObservableProperty] private bool isLiked;
    public ObservableCollection<MusicItem> MusicItems = new();
    private readonly int musicItemGroupCount = 30; // 一次加载的MusicItem数量
    private readonly double loadOffset = 500; // 到达底部多少距离时加载
    private int currentItemGroupIndex = 0;

    public void OnNavigatedFrom()
    {
    }

    public async void OnNavigatedTo(object parameter)
    {
        CurrentId = (long) parameter;
        if (CurrentId.ToString() == FavoritePlaylistService.Instance.FavoritePlaylistId)
        {
            if (FavoritePlaylistService.Instance.IsLikeSongsChanged)
            {
                await CacheHelper.UpdatePlaylistAsync(CurrentId + "_playlist", CurrentId.ToString());
                FavoritePlaylistService.Instance.IsLikeSongsChanged = false;
            }
        }

        PlayListObject = await CacheHelper.GetPlaylistAsync(CurrentId + "_playlist".ToString(), CurrentId.ToString());
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
            Cover = CacheHelper.GetImageBrush(PlayListObject.CacheCoverId, PlayListObject.CoverUrl);
            IsLiked = SavedPlaylistService.Instance.IsLiked(CurrentId);
        });
    }

    private async Task InitMusicsAsync()
    {
        if (PlayListObject.Musics == null)
        {
            var elapsed = await Tools.MeasureExecutionTimeAsync(PlayListObject.InitMusicsAsync());
            Debug.WriteLine($"初始化歌单音乐耗时: {elapsed.TotalMilliseconds}ms");
        }

        for (int i = 0; i < PlayListObject.Musics.Length; i++)
        {
            var index = i;
            if (index < musicItemGroupCount)
            {
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    MusicItems.Add(new MusicItem
                    {
                        Music = PlayListObject.Musics[index],
                        Index = (index + 1).ToString("D2"),
                    });
                });
            }
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
            {
                await LoadMusicItemsByGroup();
            }
        }
    }

    /// <summary>
    /// 用于分组加载MusicItem
    /// </summary>
    private async Task LoadMusicItemsByGroup()
    {
        for (int i = 0; i < musicItemGroupCount; i++)
        {
            var index = currentItemGroupIndex + i + 1;
            if (index < PlayListObject.MusicsCount)
            {
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    MusicItems.Add(new MusicItem
                    {
                        Music = PlayListObject.Musics[index],
                        Index = (index + 1).ToString("D2"),
                    });
                });
            }
        }

        currentItemGroupIndex += musicItemGroupCount;
    }


    [RelayCommand]
    private void PlayAll() => PlayQueue.Instance.AddMusicList(playListObject.Musics);


    public void DoubleClick(object sender, DoubleTappedRoutedEventArgs e)
    {
        var listView = sender as ListView;
        if (listView.SelectedItem is MusicItem item)
        {
            PlayQueue.Instance.Play(item.Music);
            //TODO: 设置是否将歌曲添加到播放队列
            if (PlayQueue.Instance.Count == 0)
            {
                PlayQueue.Instance.AddMusicList(playListObject.Musics);
            }
        }
    }
}