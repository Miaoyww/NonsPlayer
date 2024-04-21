using System.Diagnostics;
using System.Text.Json.Serialization;
using System.Timers;
using NAudio.Utils;
using NAudio.Wave;
using NAudio.Wave.SampleProviders;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Exceptions;
using NonsPlayer.Core.Models;
using Exception = System.Exception;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Core.Nons.Player;

public class Player
{
    #region 事件

    public delegate void MusicChanged(IMusic currentMusic);

    public delegate void PlayStateChanged(bool isPlaying);

    public delegate void PositionChanged(TimeSpan time);

    public PlayStateChanged PlayStateChangedHandle;
    public PositionChanged PositionChangedHandle;
    public MusicChanged MusicChangedHandle;

    #endregion

    public bool IsMixed = false;
    public MediaFoundationReader? CurrentReader;
    private HttpClient _httpClient = new();
    public WaveOutEvent OutputDevice;
    public IWavePlayer WaveOut;

    public IMusic CurrentMusic;
    private float _volume;
    private TimeSpan _position;
    public IMusic PreviousMusic;
    public Queue<MusicMixer> _queue;
    public TimeSpan Position => CurrentReader?.CurrentTime ?? TimeSpan.Zero;

    public Player()
    {
        OutputDevice = new WaveOutEvent();
        var timer = new Timer();
        _queue = new();
        timer.Interval = 1;
        timer.Elapsed += GetCurrentInfo;
        timer.Start();
    }

    public static Player Instance { get; } = new();


    public void SetPosition(TimeSpan value)
    {
        if (CurrentReader == null) return;
        CurrentReader.CurrentTime = value;
        _position = value;
    }

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

    public async Task NewPlay(IMusic music)
    {
        if (music is LocalMusic localMusic)
        {
            await PlayCore(localMusic);
        }
        else
        {
            await Task.WhenAll(((Music)music).GetLyric(), ((Music)music).GetFileInfo());
            await PlayCore((Music)music);
        }
    }

    public async Task Play(bool rePlay = false)
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

    private async Task PlayCore(IMusic music)
    {
        if (OutputDevice == null) OutputDevice = new WaveOutEvent();
        EnqueueTrack(music);
        CurrentMusic = music;
        MusicChangedHandle?.Invoke(music);
    }

    // 使用ConcatenatingSampleProvider做到无缝切换
    public void EnqueueTrack(IMusic music)
    {
        _queue.Enqueue(new MusicMixer
        {
            Reader = new MediaFoundationReader(music.Uri),
            Music = music
        });
        if (_queue.Count == 1 && CurrentReader == null)
        {
            LoadNextTrack();
        }
        else
        {
            IsMixed = true;
        }
        PlayQueue.Instance.Add(music);
    }

    public void LoadNextTrack()
    {
        if (_queue.Count > 0)
        {
            if (CurrentReader != null)
            {
                CurrentReader.Dispose();
            }

            if (OutputDevice.PlaybackState == PlaybackState.Playing)
            {
                OutputDevice.Stop();
            }
            var nextTrack = _queue.Dequeue();
            CurrentReader = nextTrack.Reader;
            CurrentMusic = nextTrack.Music;
            MusicChangedHandle?.Invoke(CurrentMusic);
            OutputDevice.Init(CurrentReader);
            OutputDevice.Play();
        }
        else
        {
            IsMixed = false;
        }
    }

//     if (MediaFoundationReader == null) MediaFoundationReader = new MediaFoundationReader(music.Uri);
//
//     if (music2play.Uri != null)
//     {
//         IsInitializingNewMusic = true;
//         OutputDevice.Stop();
//         OutputDevice.Dispose();
//         MediaFoundationReader = new MediaFoundationReader(music2play.Uri);
//         OutputDevice.Init(MediaFoundationReader);
//     }
//
//     if (music2play.Uri == null)
//         throw new MusicUrlNullException($"此音乐{music2play.Name} - {music2play.ArtistsName}的Url为空,可能是音乐源");
//     MusicChangedHandle?.Invoke(music2play);
//     CurrentMusic = music2play;
//     OutputDevice.Play();
//     await Task.Run(async () =>
//     {
//         await Task.Delay(1000);
//         IsInitializingNewMusic = false;
//     });
// }


// private void PlayCore(IWaveProvider waveProvider)
// {
// }
    /// <summary>
    ///     用于获取播放器当前信息
    /// </summary>
    private void GetCurrentInfo(object? sender, ElapsedEventArgs e)
    {
        if (OutputDevice != null && PlayStateChangedHandle != null)
        {
            if (CurrentReader != null)
            {
                if (CurrentReader.Position >= CurrentReader.Length)
                {
                    LoadNextTrack();
                }
            }


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

    /// <summary>
    ///     播放一个新的音乐
    /// </summary>
    /// <param name="music2play">即将播放的音乐</param>
// public async Task NewPlay(IMusic music2play)
// {
//     if (music2play is LocalMusic localMusic)
//     {
//         await PlayCore(localMusic);
//     }
//     else
//     {
//         await Task.WhenAll(((Music)music2play).GetLyric(), ((Music)music2play).GetFileInfo());
//         await PlayCore((Music)music2play);
//     }
// }
//
// private async Task PlayCore(IMusic music2play)
// {
//     if (OutputDevice == null) OutputDevice = new WaveOutEvent();
//     if (MediaFoundationReader == null) MediaFoundationReader = new MediaFoundationReader(music2play.Uri);
//
//     if (music2play.Uri != null)
//     {
//         IsInitializingNewMusic = true;
//         OutputDevice.Stop();
//         OutputDevice.Dispose();
//         MediaFoundationReader = new MediaFoundationReader(music2play.Uri);
//         OutputDevice.Init(MediaFoundationReader);
//     }
//
//     if (music2play.Uri == null)
//         throw new MusicUrlNullException($"此音乐{music2play.Name} - {music2play.ArtistsName}的Url为空,可能是音乐源");
//     MusicChangedHandle?.Invoke(music2play);
//     CurrentMusic = music2play;
//     OutputDevice.Play();
//     await Task.Run(async () =>
//     {
//         await Task.Delay(1000);
//         IsInitializingNewMusic = false;
//     });
// }
//
// public void MixPlay(ISampleProvider[] providers)
// {
//     if (OutputDevice == null) OutputDevice = new WaveOutEvent();
//
//     var _concatenator = new ConcatenatingSampleProvider(providers);
//     MediaFoundationReader = new MediaFoundationReader();
//     OutputDevice.Stop();
//     OutputDevice.Dispose();
//     OutputDevice.Init(_concatenator);
//     OutputDevice.Play();
//     IsMixed = true;
// }
//
// /// <summary>
// ///     播放音乐
// /// </summary>
// /// <param name="rePlay">是否从头播放</param>
// public async void Play(bool rePlay = false)
// {
//     if (CurrentMusic == null) return;
//     try
//     {
//         if (rePlay)
//         {
//             OutputDevice.Pause();
//             OutputDevice.Stop();
//             SetPosition(TimeSpan.Zero);
//             PositionChangedHandle(TimeSpan.Zero);
//             await Task.Delay(500);
//             OutputDevice.Play();
//             PlayStateChangedHandle(true);
//             return;
//         }
//
//         if (OutputDevice.PlaybackState == PlaybackState.Paused ||
//             OutputDevice.PlaybackState == PlaybackState.Stopped)
//         {
//             OutputDevice.Play();
//             PlayStateChangedHandle(true);
//         }
//         else
//         {
//             OutputDevice.Pause();
//             PlayStateChangedHandle(false);
//         }
//     }
//     catch (InvalidOperationException e)
//     {
//         Debug.WriteLine(e);
//     }
// }
}

public class MusicMixer
{
    public MediaFoundationReader Reader;
    public IMusic Music;
}