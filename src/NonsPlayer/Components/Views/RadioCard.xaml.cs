using System.Windows.Forms;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Animation;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;
using System.Collections.ObjectModel;
using UserControl = Microsoft.UI.Xaml.Controls.UserControl;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class RadioCard : UserControl
{
    public ObservableCollection<MetadataItem> Artists = new();
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private string title;
    [ObservableProperty] private IAdapter currentAdapter;
    [ObservableProperty] private bool currentSongLiked = false;
    private RadioService radioService = App.GetService<RadioService>();
    private List<IMusic> SavedSongs;
    [ObservableProperty] private IMusic currentSong;

    private CancellationTokenSource tokenSource;
    private CancellationToken cancellationToken;

    public IAdapter Adapter
    {
        set => CurrentAdapter = value;
    }


    public MusicStateModel MusicStateModel => MusicStateModel.Instance;

    public RadioCard()
    {
        ViewModel = App.GetService<RadioCardViewModel>();
        InitializeComponent();
        tokenSource = new();
        cancellationToken = tokenSource.Token;
        MusicStateModel.CurrentSongLikedChanged += MusicStateModelOnCurrentSongLikedChanged;
    }

    private void MusicStateModelOnCurrentSongLikedChanged(bool value)
    {
        if (MusicStateModel.CurrentMusic.Id.Equals(CurrentSong.Id))
        {
            CurrentSongLiked = value;
        }
    }

    public RadioCardViewModel ViewModel { get; }


    async partial void OnCurrentAdapterChanged(IAdapter value)
    {
        if (value != null)
        {
            var song = radioService.GetCurrentMusic();
            radioService.RadioMusicChanged += RadioMusicChanged;
            if (song != null)
            {
                await RefreshInfo(song);
                return;
            }

            Adapter = value;
            SavedSongs = (await value.Common.GetRadioSong()).ToList();
            if (SavedSongs == null) return;

            var firstSong = SavedSongs[0];
            await RefreshInfo(firstSong);
        }
    }

    partial void OnCurrentSongChanged(IMusic value)
    {
        CurrentSongLiked = value.IsLiked;
    }

    private async void RadioMusicChanged(IMusic music)
    {
        await RefreshInfo(music);
    }

    private async Task RefreshInfo(IMusic music)
    {
        CurrentSong = music;
        var cover = (await CacheHelper.GetImageBrushAsync(CurrentSong.CacheAvatarId,
            CurrentSong.GetCoverUrl("?param=200x200")));
        DispatcherQueue.TryEnqueue(() =>
        {
            Title = CurrentSong.Name;
            Cover = cover;
            Artists.Clear();
            foreach (var artist in CurrentSong.Artists)
            {
                Artists.Add(new MetadataItem
                {
                    Label = artist.Name,
                    Command = ForwardArtistCommand,
                    CommandParameter = artist
                });
            }
        });

    }


    [RelayCommand]
    public async Task Like()
    {
        var currentState = await CurrentAdapter.Account.IsLikedSong(CurrentSong.Id);
        await CurrentSong.Like(!currentState);
        if (MusicStateModel.CurrentMusic.Id.Equals(CurrentSong.Id))
        {
            var state = await CurrentAdapter.Account.IsLikedSong(CurrentSong.Id);
            CurrentSongLiked = state;
            CurrentSong.IsLiked = state;
            MusicStateModel.CurrentSongLiked = state;
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
        if (SavedSongs == null) return;
        if (radioService.IsStarted)
        {
            await Player.Instance.Play();
        }
        else
        {
            radioService.Start(SavedSongs.ToArray(), CurrentAdapter);
            SavedSongs.Clear();
        }
    }
}