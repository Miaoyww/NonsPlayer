using System.Diagnostics;
using System.Text.Json.Serialization;
using System.Timers;
using NAudio.Utils;
using NAudio.Wave;
using NonsPlayer.Core.Exceptions;
using NonsPlayer.Core.Models;
using Exception = System.Exception;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Core.Nons.Player;

public class Player
{
    public delegate void MusicChanged(Music currentMusic);

    public delegate void MusicStopped();

    public delegate void PlayStateChanged(bool isPlaying);

    public delegate void PositionChanged(TimeSpan time);

    private MediaFoundationReader _mfr;

    private float _volume;

    [JsonPropertyName("currentMusic")] public Music CurrentMusic;
    public MusicChanged MusicChangedHandle;

    public MusicStopped MusicStoppedHandle;
    public WaveOutEvent OutputDevice;
    public PlayStateChanged PlayStateChangedHandle;
    public PositionChanged PositionChangedHandle;
    public Music PreviousMusic;

    public Player()
    {
        OutputDevice = new WaveOutEvent();
        var timer = new Timer();
        timer.Interval = 10;
        timer.Elapsed += GetCurrentInfo;
        timer.Start();
        var dataWriter = new Timer();
        dataWriter.Interval = 100;
        timer.Elapsed += WriteCurrentInfo;
    }

    public static Player Instance { get; } = new();

    [JsonPropertyName("position")]
    public TimeSpan Position
    {
        get
        {
            try
            {
                var tempPosition = OutputDevice.GetPositionTimeSpan();
                return tempPosition;
            }
            catch (Exception e)
            {
                return TimeSpan.Zero;
            }
        }
        set
        {
            if (_mfr == null) return;
            _mfr.CurrentTime = value;
        }
    }

    [JsonPropertyName("volume")]
    public float Volume
    {
        get => _volume;
        set
        {
            if (_volume - value <= 0)
            {
                _volume = 0;
            }
            OutputDevice.Volume = value;
            _volume = value;
        }
    }

    public bool IsInitializingNewMusic { get; set; }

    /// <summary>
    ///     用于获取播放器当前信息
    /// </summary>
    private void GetCurrentInfo(object? sender, ElapsedEventArgs e)
    {
        if (OutputDevice != null && PlayStateChangedHandle != null)
        {
            if (OutputDevice.PlaybackState == PlaybackState.Playing)
            {
                var playerState = new PlayerState
                {
                    Position = Position
                };
                PositionChangedHandle(playerState.Position);
                PlayStateChangedHandle(true);
            }

            if (OutputDevice.PlaybackState == PlaybackState.Paused ||
                OutputDevice.PlaybackState == PlaybackState.Stopped)
                PlayStateChangedHandle(false);
        }
    }

    //TODO: 为播放器添加一个缓冲区，用于存储播放器的信息，当播放器停止时，将缓冲区的信息写入文件
    private void WriteCurrentInfo(object? sender, ElapsedEventArgs e)
    {
        if (OutputDevice != null && PlayStateChangedHandle != null)
            if (OutputDevice.PlaybackState == PlaybackState.Playing)
            {
            }
    }

    /// <summary>
    ///     播放一个新的音乐
    /// </summary>
    /// <param name="music2play">即将播放的音乐</param>
    public async Task NewPlay(Music music2play)
    {
        if (OutputDevice == null) OutputDevice = new WaveOutEvent();
        await Task.WhenAll(music2play.GetLyric(), music2play.GetFileInfo());
        MusicChangedHandle(music2play);
        CurrentMusic = music2play;
        if (_mfr == null) _mfr = new MediaFoundationReader(music2play.Url);

        if (music2play.Url != null)
        {
            IsInitializingNewMusic = true;
            OutputDevice.Stop();
            OutputDevice.Dispose();
            _mfr = new MediaFoundationReader(music2play.Url);
            OutputDevice.Init(_mfr);
        }

        if (music2play.Url == null)
            throw new MusicUrlNullException($"此音乐{music2play.Name} - {music2play.ArtistsName}的Url为空,可能是音乐源");

        OutputDevice.Play();
        await Task.Run(async () =>
        {
            await Task.Delay(1000);
            IsInitializingNewMusic = false;
        });
    }

    /// <summary>
    ///     播放音乐
    /// </summary>
    /// <param name="rePlay">是否从头播放</param>
    public async void Play(bool rePlay = false)
    {
        if (CurrentMusic == null) return;
        try
        {
            if (rePlay)
            {
                OutputDevice.Pause();
                OutputDevice.Stop();
                Position = TimeSpan.Zero;
                PositionChangedHandle(TimeSpan.Zero);
                await Task.Delay(500);
                OutputDevice.Play();
                PlayStateChangedHandle(true);
                return;
            }

            if (OutputDevice.PlaybackState == PlaybackState.Paused ||
                OutputDevice.PlaybackState == PlaybackState.Stopped)
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
        catch (InvalidOperationException e)
        {
            Debug.WriteLine(e);
        }
    }
}