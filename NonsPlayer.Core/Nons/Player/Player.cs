using System.Diagnostics;
using System.Text.Json.Serialization;
using System.Timers;
using NAudio.Utils;
using NAudio.Wave;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Exceptions;
using NonsPlayer.Core.Models;
using Exception = System.Exception;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Core.Nons.Player;

public class Player
{
    public delegate void MusicChanged(IMusic currentMusic);

    public delegate void MusicStopped();

    public delegate void PlayStateChanged(bool isPlaying);

    public delegate void PositionChanged(TimeSpan time);

    public MediaFoundationReader NPMediaFoundationReader;

    private float _volume;

    [JsonPropertyName("currentMusic")] public IMusic CurrentMusic;
    public MusicChanged MusicChangedHandle;

    public MusicStopped MusicStoppedHandle;
    public WaveOutEvent OutputDevice;
    public PlayStateChanged PlayStateChangedHandle;
    public PositionChanged PositionChangedHandle;
    public IMusic PreviousMusic;
    private TimeSpan _position;

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
        get => NPMediaFoundationReader.CurrentTime;
    }

    public void SetPosition(TimeSpan value)
    {
        if (NPMediaFoundationReader == null) return;
        NPMediaFoundationReader.CurrentTime = value;
        _position = value;
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
                PositionChangedHandle(Position);
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
    public async Task NewPlay(IMusic music2play)
    {
        if (music2play is LocalMusic localMusic)
        {
            await NewPlay(localMusic);
        }
        else
        {
            await NewPlay((Music)music2play);
            await Task.WhenAll(((Music)music2play).GetLyric(), ((Music)music2play).GetFileInfo());

        }
        await PlayCore(music2play);
    }

    private async Task PlayCore(IMusic music2play)
    {
        if (OutputDevice == null) OutputDevice = new WaveOutEvent();
        if (NPMediaFoundationReader == null) NPMediaFoundationReader = new MediaFoundationReader(music2play.Uri);

        if (music2play.Uri != null)
        {
            IsInitializingNewMusic = true;
            OutputDevice.Stop();
            OutputDevice.Dispose();
            NPMediaFoundationReader = new MediaFoundationReader(music2play.Uri);
            OutputDevice.Init(NPMediaFoundationReader);
        }

        if (music2play.Uri == null)
            throw new MusicUrlNullException($"此音乐{music2play.Name} - {music2play.ArtistsName}的Url为空,可能是音乐源");
        MusicChangedHandle?.Invoke(music2play);
        CurrentMusic = music2play;
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
                SetPosition(TimeSpan.Zero);
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