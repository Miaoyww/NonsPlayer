using System.ComponentModel;
using System.Runtime.CompilerServices;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;
using Windows.UI;
using NonsPlayer.Services;

namespace NonsPlayer.Data;

public class MusicState : INotifyPropertyChanged
{
    public static MusicState Instance
    {
        get;
    } = new();

    private MusicState()
    {
        Player.Instance.Position = TimeSpan.Zero;
        CurrentMusic = new Music();
        Cover = new SolidColorBrush(Color.FromArgb(230, 230, 230, 230));
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }

    private double _previousVolume; // 用于储存mute之前的音量，以便恢复
    private double _currentVolume; // 当前的音量
    private bool _isPlaying;
    private Brush _cover;
    private TimeSpan _duration = TimeSpan.Zero;
    private TimeSpan _position = TimeSpan.Zero;

    private Music _currentMusic;

    public Music CurrentMusic
    {
        set
        {
            _currentMusic = value;
            if (_currentMusic.IsEmpty)
            {
                return;
            }

            ImageBrush brush = new()
            {
                ImageSource = new BitmapImage(new Uri(_currentMusic.CoverUrl))
            };
            Cover = brush;
            Duration = _currentMusic.TotalTime;
            OnPropertyChanged();
            OnPropertyChanged(nameof(Cover));
        }
        get => _currentMusic;
    }

    public Brush Cover
    {
        set => _cover = value;
        get
        {
            if (CurrentMusic.IsEmpty)
            {
                return _cover;
            }
            else
            {
                return new ImageBrush
                {
                    ImageSource = new BitmapImage(new Uri(CurrentMusic.CoverUrl + "?param=300y300"))
                };
            }
        }
    }

    public TimeSpan Duration
    {
        get => CurrentMusic == null ? TimeSpan.Zero : _position;
        set
        {
            _duration = value;
            OnPropertyChanged(nameof(Duration));
            OnPropertyChanged(nameof(DurationString));
            OnPropertyChanged(nameof(DurationDouble));
        }
    }

    public string DurationString
    {
        get
        {
            if (_position.Equals(null))
            {
                return "00:00";
            }

            return _duration.ToString(@"mm\:ss");
        }
    }

    public double DurationDouble
    {
        get
        {
            if (_duration.Equals(null))
            {
                return 0.0;
            }

            return _duration.TotalSeconds;
        }
    }

    public bool IsPlaying
    {
        set
        {
            _isPlaying = value;
            OnPropertyChanged(nameof(IsPlaying));
        }
        get => _isPlaying;
    }

    public double PreviousVolume
    {
        get => _previousVolume;
        set => _previousVolume = value;
    }

    public double Volume
    {
        set
        {
            _currentVolume = value;
            RegHelper.Instance.Set(RegHelper.Regs.Volume, value.ToString());
            if (Player.Instance.OutputDevice != null)
            {
                Player.Instance.Volume = (float)value / 100;
            }

            OnPropertyChanged(nameof(Volume));
        }
        get => _currentVolume;
    }

    public TimeSpan Position
    {
        set
        {
            _position = value;

            OnPropertyChanged(nameof(Position));
            OnPropertyChanged(nameof(PositionString));
            OnPropertyChanged(nameof(PositionDouble));
        }
        get => CurrentMusic == null ? TimeSpan.Zero : _position;
    }

    public string PositionString
    {
        get
        {
            if (_position.Equals(null))
            {
                return "00:00";
            }

            return _position.ToString(@"mm\:ss");
        }
    }

    public double PositionDouble
    {
        set
        {
            Position = TimeSpan.FromSeconds(value);
        }
        get
        {
            if (_position.Equals(null))
            {
                return 0.0;
            }

            return _position.TotalSeconds;
        }
    }
}