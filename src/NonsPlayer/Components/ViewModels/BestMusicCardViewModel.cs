using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using System.Collections.ObjectModel;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class BestMusicCardViewModel
{
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private string? id;
    [ObservableProperty] private string subtitle;
    [ObservableProperty] private string title;
    [ObservableProperty] private IMusic currentMusic;
    public ObservableCollection<MetadataItem> ArtistsMetadata = new();

    async partial void OnCurrentMusicChanged(IMusic value)
    {
        Id = value.Id;
        Title = value.Name;
        Subtitle = value.ArtistsName;
        Cover = await CacheHelper.GetImageBrushAsync(value.Album.CacheMiddleAvatarId, value.Album.MiddleAvatarUrl);

        foreach (var artist in value.Artists)
        {
            ArtistsMetadata.Add(new MetadataItem
            {
                Label = artist.Name,
                Command = ForwardArtistCommand,
                CommandParameter = artist
            });
        }
    }

    [RelayCommand]
    private void ForwardArtist(IArtist artist)
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(ArtistViewModel)?.FullName, artist);
    }

    [RelayCommand]
    public void Play()
    {
        PlayQueue.Instance.Play(CurrentMusic);
    }
}