using System.Collections.ObjectModel;
using Windows.UI;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.ViewModels;

public partial class RadioCardViewModel : ObservableObject
{
    public ObservableCollection<MetadataItem> ArtistsMetadata = new();
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private string title;
    private ICommonAdapter adapter;
    private RadioService radioService = App.GetService<RadioService>();
    private List<IMusic> SavedSongs;
    private IMusic CurrentMusic;
    public MusicStateModel MusicStateModel => MusicStateModel.Instance;

    public RadioCardViewModel()
    {
        Init();
    }

    private async void Init()
    {
        var song = radioService.GetCurrentMusic();
        radioService.RadioMusicChanged += RadioMusicChanged;
        if (song != null)
        {
            await RefreshInfo(song);
            return;
        }

        var adapters = AdapterService.Instance.GetAdaptersByType(ISubAdapterEnum.Common);
        foreach (var item in adapters)
        {
            adapter = item.Common;
            SavedSongs = (await item.Common.GetRadioSong()).ToList();
            break;
        }

        if (SavedSongs == null) return;
        
        var firstSong = SavedSongs[0];
        await RefreshInfo(firstSong);

    }

    private async void RadioMusicChanged(IMusic music)
    {
        await RefreshInfo(music);
    }

    private async Task RefreshInfo(IMusic music)
    {
        CurrentMusic = music;
        Title = CurrentMusic.Name;
        Cover = (await CacheHelper.GetImageBrushAsync(CurrentMusic.CacheAvatarId,
            CurrentMusic.GetCoverUrl("?param=200x200")));
        ArtistsMetadata.Clear();
        foreach (var artist in CurrentMusic.Artists)
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
    public async void Play()
    {
        if (radioService.IsStarted)
        {
            await Player.Instance.Play();
        }
        else
        {
            radioService.Start(SavedSongs.ToArray(), adapter);
            SavedSongs.Clear();
        }
    }
}