using System.Collections.ObjectModel;
using Windows.ApplicationModel.DataTransfer;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.ViewModels;

public partial class PlaylistMusicItemCardViewModel : ObservableObject
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
    [ObservableProperty] private string trans;
    [ObservableProperty] private Visibility transVisibility;
    public ObservableCollection<MetadataItem> ArtistsMetadata = new();

    [RelayCommand]
    private void ForwardArtist(Artist artist)
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(ArtistViewModel)?.FullName, artist);
    }    
    [RelayCommand]
    private void ForwardAlbum()
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(AlbumViewModel)?.FullName, Music.Album);
    }

    public void Init(Music music)
    {
        Music = music;
        Name = Music.Name;
        Time = Music.TotalTimeString;
        Album = Music.AlbumName;
        foreach (var artist in music.Artists)
        {
            ArtistsMetadata.Add(new MetadataItem
            {
                Label = artist.Name,
                Command = ForwardArtistCommand,
                CommandParameter = artist
            });
        }

        Artists = string.IsNullOrEmpty(Music.ArtistsName) ? "未知艺人" : Music.ArtistsName;


        Trans = $"({Music.Trans})";
        if (Music.Trans.Equals(string.Empty))
            TransVisibility = Visibility.Collapsed;
        else
            TransVisibility = Visibility.Visible;

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

    [RelayCommand]
    public void PlayNext()
    {
        PlayQueue.Instance.AddNext(Music);
    }

    [RelayCommand]
    public void CheckAlbum()
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(AlbumViewModel)?.FullName, Music.Album);
    }

    [RelayCommand]
    public void CopyMusicInfo()
    {
        var data = new DataPackage();
        data.SetText($"Share music: {Music.Name} made by {Music.ArtistsName}");
        Clipboard.SetContent(data);
    }

    [RelayCommand]
    public void CopyShareUrl()
    {
        var data = new DataPackage();
        data.SetText(Music.ShareUrl);
        Clipboard.SetContent(data);
    }

    public void Play(object sender, DoubleTappedRoutedEventArgs e)
    {
        PlayQueue.Instance.Play(Music);
    }

    public async Task InitCover()
    {
        var temp = await CacheHelper.GetImageBrushAsync(Music.Album.CacheSmallAvatarId, Music.Album.SmallAvatarUrl)
            .ConfigureAwait(false);
        ServiceHelper.DispatcherQueue.TryEnqueue(() => { Cover = temp; });
    }
}