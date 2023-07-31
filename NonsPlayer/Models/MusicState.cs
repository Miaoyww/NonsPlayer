using System.ComponentModel;
using System.Runtime.CompilerServices;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;
using Windows.UI;
using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Helpers;
using NonsPlayer.Services;

namespace NonsPlayer.Models;

[INotifyPropertyChanged]
public partial class MusicState
{
    public static MusicState Instance
    {
        get;
    } = new();

    private MusicState()
    {
        Player.Instance.Position = TimeSpan.Zero;
        CurrentMusic = Music.CreateEmpty();
        Cover = new SolidColorBrush(Color.FromArgb(230, 230, 230, 230));
    }


    [ObservableProperty] private double previousVolume;
    [ObservableProperty] private double currentVolume;
    [ObservableProperty] private bool isPlaying;
    [ObservableProperty] private Brush cover;
    [ObservableProperty] private double volume;
    [ObservableProperty] private TimeSpan duration = TimeSpan.Zero;
    [ObservableProperty] private TimeSpan position = TimeSpan.Zero;
    [ObservableProperty] private Music currentMusic;

    partial void OnCurrentMusicChanged(Music value)
    {
        if (value.IsEmpty)
        {
            return;
        }
        cover = CacheHelper.GetImageBrush(value.Album.CacheCoverId, value.Album.CoverUrl);
        duration = value.TotalTime;
        OnPropertyChanged(nameof(Cover));
        OnPropertyChanged(nameof(Duration));
        OnPropertyChanged(nameof(DurationString));
    }

    partial void OnPositionChanging(TimeSpan value)
    {
        if (value.Equals(null))
        {
            return;
        }

        OnPropertyChanged(nameof(PositionString));
    }

    partial void OnDurationChanged(TimeSpan value)
    {
        if (value.Equals(null))
        {
            return;
        }

        OnPropertyChanged(nameof(DurationString));
    }

    partial void OnVolumeChanging(double value)
    {
        currentVolume = value;
        RegHelper.Instance.Set(RegHelper.Regs.Volume, value.ToString());
        if (Player.Instance.OutputDevice != null)
        {
            Player.Instance.Volume = (float)value / 100;
        }
    }

    public string DurationString
    {
        get
        {
            if (position.Equals(null))
            {
                return "00:00";
            }

            return duration.ToString(@"mm\:ss");
        }
    }

    public string PositionString
    {
        get
        {
            if (position.Equals(null))
            {
                return "00:00";
            }

            return position.ToString(@"mm\:ss");
        }
    }
}