using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.Models;
using NonsPlayer.Components.ViewModels;
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
    [ObservableProperty] private IPlaylist playList;

    [ObservableProperty] private Visibility infoVisibility = Visibility.Visible;

    public void OnNavigatedFrom()
    {
    }

    public async void OnNavigatedTo(object parameter)
    {
        PlayList = (IPlaylist)parameter;
        if (PlayList is RecommendedPlaylistCardViewModel.RecommendedPlaylist)
        {
            InfoVisibility = Visibility.Collapsed;
        }
        CurrentId = PlayList.Id;
        if (!PlayList.IsInitialized) await Task.Run(PlayList.InitializePlaylist);
        LoadPlaylistDetail();
        await Task.Run(InitMusicsAsync);
    }

    private void LoadPlaylistDetail()
    {
        Name = PlayList.Name;
        Creator = "made by " + PlayList.Creator;
        CreateTime = $"· {PlayList.CreateTime.ToString().Split(" ")[0]}";
        Description = PlayList.Description;
        MusicsCount = PlayList.MusicsCount + " Tracks";
        Cover = CacheHelper.GetImageBrush(PlayList.CacheAvatarId, PlayList.AvatarUrl);
        // IsLiked = UserPlaylistService.Instance.IsLiked(CurrentId);
    }

    private async Task InitMusicsAsync()
    {
        if (PlayList.Musics == null) await PlayList.InitializeMusics();

        for (var i = 0; i < PlayList.Musics.Count; i++)
        {
            var index = i;
            if (index < AppConfig.Instance.AppSettings.PlaylistTrackCount)
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    MusicItems.Add(new MusicModel
                    {
                        Music = PlayList.Musics[index], Index = (index + 1).ToString("D2")
                    });
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
                currentItemGroupIndex < PlayList.MusicsCount - 1)
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
            if (index < PlayList.MusicsCount)
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    MusicItems.Add(new MusicModel
                    {
                        Music = PlayList.Musics[index], Index = (index + 1).ToString("D2")
                    });
                });
        }

        currentItemGroupIndex += AppConfig.Instance.AppSettings.PlaylistTrackCount;
    }


    [RelayCommand]
    private async void PlayAll()
    {
        if (PlayList.Musics.Count != PlayList.MusicTrackIds.Length)
            // +1到达最后一个歌曲
            // await playListObject.InitTrackByIndexAsync(1000, playListObject.MusicTrackIds.Length + 1);

            PlayQueue.Instance.Clear();
        PlayQueue.Instance.AddMusicList(PlayList.Musics.ToArray(), true);
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