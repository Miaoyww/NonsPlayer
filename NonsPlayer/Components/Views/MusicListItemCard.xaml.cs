using ATL;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.Models;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class MusicListItemCard : UserControl
{
    [ObservableProperty] private string index;

    [ObservableProperty] private bool isCoverInit;

    [ObservableProperty] private IMusic music;


    public MusicListItemCard()
    {
        ViewModel = App.GetService<MusicListItemViewModel>();
        InitializeComponent();
        PropertiesFlyOut.Text = "Properties".GetLocalized();
    }

    public MusicListItemViewModel ViewModel { get; }

    partial void OnMusicChanged(IMusic music)
    {
        ViewModel.Init(music);
        for (var i = 0; i < Music.Artists.Length; i++)
        {
            CheckArtists.Items.Add(new MenuFlyoutItem
            {
                Text = Music.Artists[i].Name,
                Command = CheckArtistCommand,
                Style = App.Current.Resources["CustomMenuFlyoutItem"] as Style,
                CommandParameter = Music.Artists[i]
            });
        }
    }

    [RelayCommand]
    public async Task OpenProperties()
    {
        var dialog = new ContentDialog();
        dialog.XamlRoot = this.XamlRoot;
        dialog.Title = "Properties".GetLocalized();
        dialog.PrimaryButtonText = "Save";
        dialog.CloseButtonText = "Cancel";
        dialog.DefaultButton = ContentDialogButton.Primary;
        dialog.Content = new LocalProperties(music);
        dialog.Width = 1000;
        var result = await dialog.ShowAsync();
        if (result == ContentDialogResult.Primary)
        {
            var path = ((LocalMusic)music).FilePath;
            var model = ((LocalProperties)dialog.Content).Tag as LocalTrackModel;
            var track = new Track(path);
            if (model != null)
            {
                if (!string.IsNullOrEmpty(model.Title))
                {
                    track.Title = model.Title;
                    music.Name = model.Title;
                }

                if (!string.IsNullOrEmpty(model.Artist))
                {
                    track.Artist = model.Artist;
                    music.Artists[0].Name = model.Artist;
                }

                if (!string.IsNullOrEmpty(model.Album))
                {
                    track.Album = model.Album;
                    music.Album.Name = model.Album;
                }

                if (!string.IsNullOrEmpty(model.AlbumArtists))
                {
                    track.AlbumArtist = model.AlbumArtists;
                    music.Album.Artists = [new LocalArtist { Name = model.AlbumArtists }];
                }

                if (!string.IsNullOrEmpty(model.TrackNumber)) track.TrackNumber = Convert.ToInt32(model.TrackNumber);
                if (!string.IsNullOrEmpty(model.Genre)) track.Genre = model.Genre;
                if (!string.IsNullOrEmpty(model.Date)) track.Year = Convert.ToInt32(model.Date);
                track.Save();
                ViewModel.Init(music);
            }
        }
    }

    [RelayCommand]
    public void CheckArtist(IArtist artist)
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(ArtistViewModel)?.FullName, artist);
    }

    partial void OnIsCoverInitChanged(bool isCoverInit)
    {
        ViewModel.IsInitCover = isCoverInit;
    }

    partial void OnIndexChanged(string index)
    {
        ViewModel.Index = index;
    }

    [RelayCommand]
    private async void Like()
    {
        await Music.Like(!Music.IsLiked);
        if (MusicStateModel.Instance.CurrentMusic.Id.Equals(music.Id))
        {
            var state = await music.GetLikeState();
            Music.IsLiked = state;
            MusicStateModel.Instance.CurrentSongLiked = state;
        }

        ViewModel.Liked = await Music.GetLikeState();
    }
}