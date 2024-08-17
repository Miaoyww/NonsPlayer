using System.Collections.ObjectModel;
using Windows.UI;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.Extensions.Logging;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

[INotifyPropertyChanged]
public partial class MusicStateModel
{
    public delegate void CurrentSongLikedChangedEventHandler(bool value);

    public event CurrentSongLikedChangedEventHandler? CurrentSongLikedChanged;

    [ObservableProperty] private Brush cover;
    [ObservableProperty] private IMusic? currentMusic;
    [ObservableProperty] private bool currentSongLiked;
    [ObservableProperty] private double currentVolume;
    [ObservableProperty] private TimeSpan duration = TimeSpan.Zero;
    [ObservableProperty] private bool isPlaying;
    [ObservableProperty] private bool onDrag;
    [ObservableProperty] private bool showCover;
    public ObservableCollection<MetadataItem> ArtistsMetadata = new();
    private double position;
    [ObservableProperty] private double previousVolume;
    [ObservableProperty] private double volume;

    private ILogger logger = App.GetLogger<MusicStateModel>();

    private MusicStateModel()
    {
        CurrentMusic = null;
        Volume = ConfigManager.Instance.Settings.Volume;
        ShowCover = false;
    }

    public static MusicStateModel Instance { get; } = new();

    partial void OnCurrentSongLikedChanged(bool value)
    {
        CurrentSongLikedChanged?.Invoke(value);
        if (CurrentMusic != null)
        {
            CurrentMusic.IsLiked = value;
        }
    }

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
    private void ForwardArtist(IArtist artist)
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(ArtistViewModel)?.FullName, artist);
    }

    async partial void OnCurrentMusicChanged(IMusic value)
    {
        if (value.IsEmpty) return;
        try
        {
            if (value is LocalMusic)
            {
                Cover = await CacheHelper.GetImageBrushAsync(value.Album.CacheAvatarId, value.LocalCover);
            }
            else
            {
                // Cover = new ImageBrush { ImageSource = new BitmapImage(new Uri(value.GetCoverUrl("?param=50x50"))) };
                Cover = CacheHelper.GetImageBrush(value.Album.CacheAvatarId, value.GetCoverUrl("?param=50x50"));
            }

            ShowCover = true;
        }
        catch (Exception e)
        {
            // 此为无封面
            ShowCover = false;
        }

        Duration = value.Duration;
        ArtistsMetadata.Clear();
        foreach (var artist in value.Artists)
        {
            ArtistsMetadata.Add(new MetadataItem
            {
                Label = artist.Name, Command = ForwardArtistCommand, CommandParameter = artist
            });
        }

        CurrentSongLiked = value.IsLiked;

        if (CurrentMusic is LocalMusic)
        {
            logger.LogInformation(
                $"Play new song: [{value.Id}] {value.Name} - {value.ArtistsName} from local music");
        }
        else
        {
            logger.LogInformation(
                $"Play new song: [{value.Id}] {value.Name} - {value.ArtistsName} from adapter: {value.Adapter.GetMetadata().DisplayPlatform}");
        }
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
        ConfigManager.Instance.Settings.Volume = value;
        if (Player.Instance.OutputDevice != null) Player.Instance.Volume = (float)value / 100;
    }
}