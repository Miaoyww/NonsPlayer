using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using System.Collections.ObjectModel;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class FavoriteSongCardViewModel
{
    [ObservableProperty] private IMusic music;
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private string title;
    public ObservableCollection<MetadataItem> ArtistsMetadata = new();

    async partial void OnMusicChanged(IMusic value)
    {
        Cover = await CacheHelper.GetImageBrushAsync(value.Album.CacheSmallAvatarId,
            value.GetCoverUrl("?param=50x50"));
        Title = value.Name;
        foreach (var artist in music.Artists)
        {
            ArtistsMetadata.Add(new MetadataItem
            {
                Label = artist.Name, Command = ForwardArtistCommand, CommandParameter = artist
            });
        }
    }

    [RelayCommand]
    private void ForwardArtist(IArtist artist)
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(ArtistViewModel)?.FullName, artist);
    }
}