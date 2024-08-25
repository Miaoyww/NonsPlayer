using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
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