﻿using System.ComponentModel;
using System.Runtime.CompilerServices;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;
using Windows.UI;
using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;

namespace NonsPlayer.Models;

[INotifyPropertyChanged]
public partial class MusicState
{
    public static MusicState Instance { get; } = new();

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
    [ObservableProperty] private Music currentMusic;
    [ObservableProperty] private bool currentSongLiked;
    [ObservableProperty] private bool onDrag;
    private double position;
    
    partial void OnCurrentMusicChanged(Music value)
    {
        if (value.IsEmpty)
        {
            return;
        }

        cover = CacheHelper.GetImageBrush(value.Album.CacheCoverId, value.Album.CoverUrl);
        duration = value.TotalTime;
        CurrentSongLiked = FavoritePlaylistService.Instance.IsLiked(value.Id);
        OnPropertyChanged(nameof(Cover));
        OnPropertyChanged(nameof(Duration));
        OnPropertyChanged(nameof(DurationString));
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
            Player.Instance.Volume = (float) value / 100;
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

            return TimeSpan.FromSeconds(position).ToString(@"mm\:ss");
        }
    }
}