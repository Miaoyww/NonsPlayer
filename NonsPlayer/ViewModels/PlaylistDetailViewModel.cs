using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Cache;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;
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
    private bool isLoadingImages = false;
    public ObservableCollection<MusicItem> MusicItems = new();
    public List<Music> Musics = new();

    public void OnNavigatedFrom()
    {
    }

    public async void OnNavigatedTo(object parameter)
    {
        if ((long)parameter == CurrentId)
        {
            return;
        }

        CurrentId = (long)parameter;
        PlayListObject = await CacheHelper.GetPlaylistAsync(CurrentId + "_playlist".ToString(), CurrentId.ToString());
        if (PlayListObject.IsCardMode)
        {
            await PlayListObject.LoadAsync(PlayListObject.Id).ConfigureAwait(false);
        }

        LoadPlaylistDetail();

        await LoadMusicsAsync();
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
        });
    }

    private async Task LoadMusicsAsync()
    {
        if (PlayListObject.Musics == null)
        {
            await PlayListObject.InitMusicsAsync();
        }

        for (var i = 0; i < PlayListObject.MusicsCount; i++)
        {
            Musics.Add(PlayListObject.Musics[i]);
            var index = i;
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                MusicItems.Add(new MusicItem
                {
                    Music = PlayListObject.Musics[index],
                    Index = (index + 1).ToString("D2"),
                    IsInitCover = false
                });
            });
        }
    }

    public async void OnScrollViewerViewChanged(object? sender, ScrollViewerViewChangedEventArgs e)
    {
        if (sender is ScrollViewer scrollViewer)
        {
            // 获取可见的第一个和最后一个项的索引
            var firstVisibleIndex = (int)scrollViewer.VerticalOffset;
            var lastVisibleIndex = (int)(scrollViewer.VerticalOffset + scrollViewer.ViewportHeight);

            // 加载可见项的图片
            await LoadVisibleItemsImages(firstVisibleIndex, lastVisibleIndex);
        }
    }

    private async Task LoadVisibleItemsImages(int firstIndex, int lastIndex)
    {
        if (isLoadingImages)
        {
            // 如果正在加载图片，则不重复处理
            return;
        }

        isLoadingImages = true;

        // 确保索引不超出项的范围
        var itemCount = Musics.Count;
        firstIndex = Math.Max(0, firstIndex);
        lastIndex = Math.Min(itemCount - 1, lastIndex);

        // 加载可见项的图片
        for (int i = firstIndex; i <= lastIndex; i++)
        {
            var item = MusicItems[i];

            // 检查图片是否已加载
            if (item.Music.Album.SmallCoverUrl != null)
            {
                // TODO: 完善图片加载逻辑
                item.IsInitCover = true;
            }
        }

        isLoadingImages = false;
    }


    [RelayCommand]
    private void PlayAll() => PlayQueue.Instance.AddMusicList(Musics);
}