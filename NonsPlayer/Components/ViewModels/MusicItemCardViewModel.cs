using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.ViewModels;

public partial class MusicItemCardViewModel : ObservableObject
{
    [ObservableProperty] private string album;
    [ObservableProperty] private string artists;
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private string index;
    [ObservableProperty] private bool isInitCover;
    [ObservableProperty] private bool liked;
    public Music Music;
    [ObservableProperty] private string name;
    [ObservableProperty] private string time;

    public void Init(Music music)
    {
        Music = music;
        Name = Music.Name;
        Time = Music.TotalTimeString;
        Album = Music.AlbumName;
        Artists = string.IsNullOrEmpty(Music.ArtistsName) ? "未知艺人" : Music.ArtistsName;
        Liked = FavoritePlaylistService.Instance.IsLiked(Music.Id);
        InitCover().ConfigureAwait(false);
        FavoritePlaylistService.Instance.LikeSongsChanged += () =>
        {
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                if (Liked = FavoritePlaylistService.Instance.IsLiked(Music.Id)) return;
                Liked = FavoritePlaylistService.Instance.IsLiked(Music.Id);
            });
        };
    }

    public async Task InitCover()
    {
        var temp = await CacheHelper.GetImageBrushAsync(Music.Album.CacheSmallCoverId, Music.Album.SmallCoverUrl);
        ServiceHelper.DispatcherQueue.TryEnqueue(() => { Cover = temp; });
    }

    [RelayCommand]
    public async void Like()
    {
        await FavoritePlaylistService.Instance.Like(Music.Id);
    }
}