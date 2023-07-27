using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Windows.Input;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using Windows.UI;
using NonsPlayer.Services;

namespace NonsPlayer.Resources;

public class GlobalMusicState : INotifyPropertyChanged
{
    public static GlobalMusicState Instance
    {
        get;
    } = new GlobalMusicState();

    public GlobalMusicState()
    {
        PositionChangedHandle = (t) => TimeSpan.Zero;
        CurrentMusic = new Music();
        Cover = new SolidColorBrush(Color.FromArgb(230, 230, 230, 230));
        MusicPlayCommand = new RelayCommand(() =>
        {
            PlayerService.Instance.Play();
        });
        VolumeMuteCommand = new RelayCommand(Mute);
        NextMusicCommand = new RelayCommand(OnNextMusic);
        PreviousMusicCommand = new RelayCommand(OnPreviousMusic);
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public delegate TimeSpan PositionChanged(TimeSpan time);

    public delegate Music MusicChanged(Music currentMusic);

    public PositionChanged PositionChangedHandle;
    public MusicChanged MusicChangedHandle;

    protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }

    private double _previousVolume; // 用于储存mute之前的音量，以便恢复
    private double _currentVolume; // 当前的音量
    private bool _isPlaying;
    private Brush _cover;
    private TimeSpan _durationTime = TimeSpan.Zero;
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
            DurationTime = _currentMusic.DuartionTime;
            MusicChangedHandle(value);
            OnPropertyChanged(nameof(CurrentMusic));
            OnPropertyChanged(nameof(Cover));
        }
        get => _currentMusic;
    }

    public Brush Cover
    {
        set
        {
            _cover = value;
        }
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

    public TimeSpan DurationTime
    {
        get => CurrentMusic == null ? TimeSpan.Zero : _position;
        set
        {
            _durationTime = value;
            OnPropertyChanged(nameof(DurationTime));
            OnPropertyChanged(nameof(DurationTimeString));
            OnPropertyChanged(nameof(DurationTimeDouble));
        }
    }

    public string DurationTimeString
    {
        get
        {
            if (_position.Equals(null))
            {
                return "00:00";
            }

            return _durationTime.ToString(@"mm\:ss");
        }
    }

    public double DurationTimeDouble
    {
        get
        {
            if (_durationTime.Equals(null))
            {
                return 0.0;
            }

            return _durationTime.TotalSeconds;
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

    public double Volume
    {
        set
        {
            _currentVolume = value;
            RegHelper.Instance.Set(RegHelper.Regs.Volume, value.ToString());
            if (PlayerService.Instance.OutputDevice != null)
            {
                PlayerService.Instance.OutputDevice.Volume = (float)value / 100;
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

    public ICommand MusicPlayCommand
    {
        get;
    }

    public ICommand NextMusicCommand
    {
        get;
    }

    public ICommand PreviousMusicCommand
    {
        get;
    }

    public ICommand VolumeMuteCommand
    {
        get;
    }

    /// <summary>
    /// 禁音
    /// </summary>
    public void Mute()
    {
        if (Volume > 0)
        {
            _previousVolume = Volume;
            Volume = 0;
        }
        else
        {
            Volume = _previousVolume;
        }
    }

    public void OnPreviousMusic() => PlaylistService.Instance.PlayPrevious();

    public void OnNextMusic() => PlaylistService.Instance.PlayNext();
}