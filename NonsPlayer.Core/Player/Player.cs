using System.Timers;
using NAudio.Utils;
using NAudio.Wave;
using NonsPlayer.Core.Models;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Core.Player;

public class Player
{
    public static Player Instance
    {
        get;
    } = new();

    private float _volume;
    private Music _currentMusic;
    private MediaFoundationReader _mfr;
    public WaveOutEvent OutputDevice;

    public delegate void PlayStateChanged(bool isPlaying);
    public delegate void PositionChanged(TimeSpan time);
    public delegate void MusicChanged(Music currentMusic);

    public PlayStateChanged PlayStateChangedHandle;
    public PositionChanged PositionChangedHandle;
    public MusicChanged MusicChangedHandle;

    public TimeSpan Position
    {
        get => OutputDevice.GetPositionTimeSpan();
        set
        {
            if (_mfr == null)
            {
                return;
            }

            _mfr.Position = value.Ticks;
        }
    }

    public float Volume
    {
        get => _volume;
        set
        {
            OutputDevice.Volume = value;
            _volume = value;
        }
    }
    
    public Player()
    {
        OutputDevice = new WaveOutEvent();
        var timer = new Timer();
        timer.Interval = 20;
        timer.Elapsed += GetCurrentInfo;
        timer.Start();
    }

    /// <summary>
    /// 用于获取播放器当前信息
    /// </summary>
    private void GetCurrentInfo(object? sender, ElapsedEventArgs e)
    {
        if (OutputDevice != null)
        {
            if (OutputDevice.PlaybackState == PlaybackState.Playing)
            {
                var playerState = new PlayerState
                {
                    Position = Position
                };
                PositionChangedHandle(playerState.Position);
            }
        }
    }

    /// <summary>
    /// 播放一个新的音乐
    /// </summary>
    /// <param name="music2play">即将播放的音乐</param>
    public async void NewPlay(Music music2play)
    {
        if (OutputDevice == null)
        {
            OutputDevice = new WaveOutEvent();
        }

        await Task.WhenAll(music2play.GetLyric(), music2play.GetFileInfo());
        MusicChangedHandle(music2play);
        _currentMusic = music2play;
        if (_mfr == null)
        {
            _mfr = new MediaFoundationReader(music2play.Url);
            OutputDevice.Init(_mfr);
        }
        else
        {
            if (music2play.Url != null)
            {
                OutputDevice.Stop();
                _mfr = new MediaFoundationReader(music2play.Url);
                OutputDevice.Init(_mfr);
            }
        }

        Play(true);
    }

    /// <summary>
    /// 播放音乐
    /// </summary>
    /// <param name="rePlay">是否从头播放</param>
    public void Play(bool rePlay = false)
    {
        if (_currentMusic == null)
        {
            return;
        }

        try
        {
            if (rePlay)
            {
                PlayStateChangedHandle(true);
                _mfr.Position = TimeSpan.Zero.Ticks;
                OutputDevice.Play();
            }
            else
            {
                if (OutputDevice.PlaybackState == PlaybackState.Paused)
                {
                    OutputDevice.Play();
                    PlayStateChangedHandle(true);
                }
                else
                {
                    OutputDevice.Pause();
                    PlayStateChangedHandle(false);
                }
            }
        }
        catch (InvalidOperationException)
        {
        }
    }
}