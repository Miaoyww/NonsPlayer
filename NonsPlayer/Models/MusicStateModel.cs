using System.Collections.ObjectModel;
using Windows.UI;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

[INotifyPropertyChanged]
public partial class MusicStateModel
{
    [ObservableProperty] private Brush cover;
    [ObservableProperty] private IMusic currentMusic;
    [ObservableProperty] private bool currentSongLiked;
    [ObservableProperty] private double currentVolume;
    [ObservableProperty] private TimeSpan duration = TimeSpan.Zero;
    [ObservableProperty] private bool isPlaying;
    [ObservableProperty] private bool onDrag;
    public ObservableCollection<MetadataItem> ArtistsMetadata = new();
    private double position;


    [ObservableProperty] private double previousVolume;
    [ObservableProperty] private double volume;

    private MusicStateModel()
    {
        CurrentMusic = Music.CreateEmpty();
        Cover = new SolidColorBrush(Color.FromArgb(230, 230, 230, 230));
    }

    public static MusicStateModel Instance { get; } = new();

    public double Position
    {
        get => position;
        set
        {
            position = value;
            OnPropertyChanged();
            OnPropertyChanged(nameof(PositionString));
        }
    }

    public string DurationString
    {
        get
        {
            if (position.Equals(null)) return "00:00";

            return duration.ToString(@"mm\:ss");
        }
    }

    public string PositionString
    {
        get
        {
            if (position.Equals(null)) return "00:00";

            return TimeSpan.FromSeconds(position).ToString(@"mm\:ss");
        }
    }

    [RelayCommand]
    private void ForwardArtist(Artist artist)
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(ArtistViewModel)?.FullName, artist);
    }

    async partial void OnCurrentMusicChanged(IMusic value)
    {
        if (value.IsEmpty) return;


        if (value is LocalMusic)
        {
            Cover = await CacheHelper.GetImageBrush(value.Album.CacheAvatarId, ((LocalMusic)value).Cover);
        }
        else
        {
            Cover = CacheHelper.GetImageBrush(value.Album.CacheAvatarId, value.Album.AvatarUrl);
        }

        Duration = value.Duration;
        CurrentSongLiked = FavoritePlaylistService.Instance.IsLiked(value.Id);
        ArtistsMetadata.Clear();
        foreach (var artist in value.Artists)
        {
            ArtistsMetadata.Add(new MetadataItem
            {
                Label = artist.Name,
                Command = ForwardArtistCommand,
                CommandParameter = artist
            });
        }

        OnPropertyChanged(nameof(Cover));
        OnPropertyChanged(nameof(Duration));
        OnPropertyChanged(nameof(DurationString));
    }

    partial void OnDurationChanged(TimeSpan value)
    {
        if (value.Equals(null)) return;

        OnPropertyChanged(nameof(DurationString));
    }

    partial void OnVolumeChanging(double value)
    {
        currentVolume = value;
        RegHelper.Instance.Set(RegHelper.Regs.Volume, value.ToString());
        if (Player.Instance.OutputDevice != null) Player.Instance.Volume = (float)value / 100;
    }
}