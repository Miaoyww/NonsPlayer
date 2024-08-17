using System.Collections.ObjectModel;
using Windows.ApplicationModel.DataTransfer;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.ViewModels;

public partial class MusicListItemViewModel : ObservableObject
{
    [ObservableProperty] private string album;
    [ObservableProperty] private string artists;
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private string index;
    [ObservableProperty] private bool isInitCover;
    [ObservableProperty] private bool liked;
    public IMusic Music;
    [ObservableProperty] private string name;
    [ObservableProperty] private string time;
    [ObservableProperty] private string trans;
    [ObservableProperty] private Visibility transVisibility;
    public ObservableCollection<MetadataItem> ArtistsMetadata = new();

    [RelayCommand]
    private void ForwardArtist(IArtist artist)
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(ArtistViewModel)?.FullName, artist);
    }

    [RelayCommand]
    private void ForwardAlbum()
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(AlbumViewModel)?.FullName, Music.Album);
    }

    public async void Init(IMusic music)
    {
        Music = music;
        if (music is LocalMusic)
        {
            if (!((LocalMusic)music).IsInit) ((LocalMusic)music).Init();
        }

        Name = Music.Name;
        Time = Music.TotalTimeString;
        Album = Music.AlbumName;
        foreach (var artist in music.Artists)
        {
            ArtistsMetadata.Add(new MetadataItem
            {
                Label = artist.Name, Command = ForwardArtistCommand, CommandParameter = artist
            });
        }

        Artists = string.IsNullOrEmpty(Music.ArtistsName) ? "未知艺人" : Music.ArtistsName;
        Trans = string.IsNullOrEmpty(Music.Trans) ? "" : $"({Music.Trans})";
        if (string.IsNullOrEmpty(Trans))
        {
            TransVisibility = Visibility.Collapsed;
        }
        else
        {
            TransVisibility = Visibility.Visible;
        }

        Liked = await Music.GetLikeState();
        Music.IsLiked = Liked;
        await InitCover();
        MusicStateModel.Instance.CurrentSongLikedChanged += InstanceOnCurrentSongLikedChanged;
    }

    private void InstanceOnCurrentSongLikedChanged(bool value)
    {
        if (MusicStateModel.Instance.CurrentMusic != null)
        {
            if (MusicStateModel.Instance.CurrentMusic.Id.Equals(Music.Id))
            {
                Liked = value;
            }
        }
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
        ImageBrush cover;
        if (Music is LocalMusic)
        {
            cover = await CacheHelper.GetImageBrushAsync(Music.Album.CacheAvatarId, Music.LocalCover);
        }
        else
        {
            cover = await CacheHelper.GetImageBrushAsync(Music.Album.CacheSmallAvatarId, Music.Album.SmallAvatarUrl)
                .ConfigureAwait(false);
        }

        ServiceHelper.DispatcherQueue.TryEnqueue(() => { Cover = cover; });
    }
}