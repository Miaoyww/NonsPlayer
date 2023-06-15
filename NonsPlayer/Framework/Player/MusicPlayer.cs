using System.ComponentModel;
using System.Timers;
using System.Windows.Input;
using Windows.UI;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NAudio.Wave;
using Timer = System.Timers.Timer;
using NonsPlayer.Framework.Model;
using System.Runtime.CompilerServices;

namespace NonsPlayer.Framework.Player;

public class MusicPlayer : INotifyPropertyChanged
{
    public DispatcherQueue Dispatcher
    {
        get;
        set;
    }

    public static PositionChanger PositionChangerHandle;
    /// <summary>
    /// 初始化播放器
    /// </summary>
    /// <param name="dispatcher">UI线程的dispatcher</param>
    public void InitPlayer(DispatcherQueue dispatcher)
    {
        outputDevice = new WaveOutEvent();
        Dispatcher = dispatcher;
        PositionChangerHandle = new((n) =>
        {
            return TimeSpan.Zero;

        });
        Name = "当前未播放";
        Artists = "无";
        Cover = new SolidColorBrush(Color.FromArgb(230, 230, 230, 230));
        MusicPlayCommand = new RelayCommand(() =>
        {
            Play();
        });
        VolumeMuteCommand = new RelayCommand(Mute);
        var timer = new Timer();
        timer.Interval = 20;
        timer.Elapsed += GetCurrentInfo;
        timer.Start();
    }

    /// <summary>
    /// 用于获取播放器当前信息
    /// 包括当前Position
    /// </summary>
    private void GetCurrentInfo(object? sender, ElapsedEventArgs e)
    {
        if (outputDevice != null)
        {
            if (outputDevice.PlaybackState == PlaybackState.Playing)
            {
                Dispatcher.TryEnqueue(() =>
                {
                    var postion = TimeSpan.FromSeconds(mfr.Position / outputDevice.OutputWaveFormat.BitsPerSample /
                        outputDevice.OutputWaveFormat.Channels * 8.0 / outputDevice.OutputWaveFormat.SampleRate);
                    Position = postion;
                    PositionChangerHandle(postion);
                });
            }
        }
    }

    /// <summary>
    /// 播放一个新的音乐
    /// </summary>
    /// <param name="music2play">即将播放的音乐</param>
    public async void NewPlay(Music music2play)
    {
        if (outputDevice == null)
        {
            outputDevice = new WaveOutEvent();
        }

        MusicNow = music2play;
        await MusicNow.GetLric();
        await MusicNow.GetFileInfo();
        if (mfr == null)
        {
            mfr = new MediaFoundationReader(music2play.Url);
            outputDevice.Init(mfr);
            outputDevice.Volume = (float)Volume / 100;
        }
        else
        {
            if (music2play.Url != null)
            {
                outputDevice.Stop();
                mfr = new MediaFoundationReader(music2play.Url);
                outputDevice.Init(mfr);
                outputDevice.Volume = (float)Volume / 100;
            }
        }
        Play(true);
    }

    /// <summary>
    /// 播放音乐
    /// </summary>
    /// <param name="re">是否从头播放</param>
    public void Play(bool re = false)
    {
        try
        {
            if (re)
            {
                IsPlaying = true;
                mfr.Position = TimeSpan.Zero.Ticks;
                outputDevice.Play();
            }
            else
            {
                if (!IsPlaying)
                {
                    outputDevice.Play();
                    IsPlaying = true;
                    // ResEntry.musicInfo.DurationTime = outputDevice.GetPositionTimeSpan
                }
                else
                {
                    outputDevice.Pause();
                    IsPlaying = false;
                    // ResEntry.musicInfo.DurationTime = player.NaturalDuration.TimeSpan;
                }
            }
        }
        catch (InvalidOperationException)
        {
        }
    }

    /// <summary>
    /// 禁音
    /// </summary>
    public void Mute()
    {
        if (Volume > 0)
        {
            _lastVolume = Volume;
            Volume = 0;
        }
        else
        {
            Volume = _lastVolume;
        }
    }

    #region 歌曲信息变量储存区

    private Music _musicNow;
    private string _artists;
    private Brush _cover;
    private string _name;
    private TimeSpan _durationTime = TimeSpan.Zero;

    public Music MusicNow
    {
        set
        {
            _musicNow = value;
            Name = _musicNow.Name;
            Artists = _musicNow.ArtistsName;
            ImageBrush brush = new()
            {
                ImageSource = new BitmapImage(new Uri(_musicNow.CoverUrl))
            };
            Cover = brush;
            DurationTime = _musicNow.DuartionTime;
        }
        get => _musicNow;
    }

    public string Name
    {
        set
        {
            _name = value;
            OnPropertyChanged(nameof(Name));
        }
        get => _name;
    }

    public string Artists
    {
        set
        {
            _artists = value;
            OnPropertyChanged(nameof(Artists));
        }
        get => _artists;
    }

    public Brush Cover
    {
        set
        {
            _cover = value;
            OnPropertyChanged(nameof(Cover));
        }
        get => _cover;
    }

    public TimeSpan DurationTime
    {
        set
        {
            _durationTime = value;
            OnPropertyChanged(nameof(DurationTime));
            OnPropertyChanged(nameof(DurationTimeDouble));
            OnPropertyChanged(nameof(DurationTimeString));
        }
        get => MusicNow == null ? TimeSpan.Zero : position;
    }


    public string DurationTimeString
    {
        get
        {
            if (position.Equals(null))
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

    #endregion

    #region 播放器信息储存区

    private double _lastVolume; // 用于储存mute之前的音量，以便恢复
    private MediaFoundationReader mfr;
    private WaveOutEvent outputDevice;
    private TimeSpan position = TimeSpan.Zero;
    private double volume; // 当前的音量
    private bool isPlaying;
    public bool IsPlaying
    {
        set
        {
            isPlaying = value;
            OnPropertyChanged(nameof(IsPlaying));
        }
        get => isPlaying;
    }
    public double Volume
    {
        set
        {
            volume = value;
            if (outputDevice != null)
            {
                outputDevice.Volume = (float)value / 100;
            }

            OnPropertyChanged(nameof(Volume));
        }
        get => volume;
    }
    public TimeSpan Position
    {
        set
        {
            position = value;
            OnPropertyChanged(nameof(Position));
            OnPropertyChanged(nameof(PositionDouble));
            OnPropertyChanged(nameof(PositionString));
        }
        get => MusicNow == null ? TimeSpan.Zero : position;
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

    public double PositionDouble
    {
        set
        {
            Position = TimeSpan.FromSeconds(value);
        }
        get
        {
            if (position.Equals(null))
            {
                return 0.0;
            }

            return position.TotalSeconds;
        }
    }

    #endregion

    #region Commands接口

    public ICommand MusicPlayCommand;
    public ICommand VolumeMuteCommand;


    #endregion

    #region PropertyChanged接口实现
    public delegate TimeSpan PositionChanger(TimeSpan time);
    public event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged([CallerMemberName] string propertyName = "")
    {
        if (PropertyChanged != null)
        {
            PropertyChanged(this, new PropertyChangedEventArgs(propertyName));
        }
    }
    #endregion
}