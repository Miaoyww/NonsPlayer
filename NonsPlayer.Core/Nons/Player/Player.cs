using System.Diagnostics;
using System.Text.Json.Serialization;
using System.Timers;
using NAudio.Utils;
using NAudio.Wave;
using NAudio.Wave.SampleProviders;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Exceptions;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
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

    public bool IsMixed;
    public MediaFoundationReader? CurrentReader;
    public VolumeSampleProvider? VolumeProvider;
    public WaveOutEvent OutputDevice;
    
    public IMusic CurrentMusic;
    private float _volume;
    private TimeSpan _position;
    public IMusic PreviousMusic;
    public Queue<MusicMixer> _queue;
    public TimeSpan Duration => CurrentMusic?.Duration ?? TimeSpan.Zero;

    /// <summary>
    /// 无缝切换的时间点 Item1: 音乐 Item2: 开始时间 Item3: 结束时间 Item4: Reader
    /// </summary>
    private List<Tuple<IMusic, TimeSpan, TimeSpan, MediaFoundationReader>>? _jointlessTimes;

    private Tuple<IMusic, TimeSpan, TimeSpan, MediaFoundationReader>? _currentJointlessTime;
    public TimeSpan Position => CurrentReader?.CurrentTime ?? TimeSpan.Zero;

    public Player()
    {
        OutputDevice = new WaveOutEvent();
        var timer = new Timer();
        _queue = new();
        timer.Interval = 10;
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

            _volume = value;
            if (VolumeProvider != null) VolumeProvider.Volume = value;
        }
    }

    public async Task NewPlay(IMusic music)
    {
        _queue.Clear();
        IsMixed = false;
        if (music is LocalMusic localMusic)
        {
            await PlayCore(localMusic);
        }
        else
        {
            await Task.WhenAll((music).GetLyric(), (music).GetUrl());
            await PlayCore(music);
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
        EnqueueTrack([music]);
        CurrentMusic = music;
    }

    // 使用ConcatenatingSampleProvider做到无缝切换
    public void EnqueueTrack(IMusic[] song)
    {
        MusicMixer mixer;
        if (song.Length != 0)
        {
            var readers = new MediaFoundationReader[song.Length];
            var providers = new ISampleProvider[song.Length];
            for (int i = 0; i < song.Length; i++)
            {
                readers[i] = new MediaFoundationReader(song[i].Url);
                providers[i] = readers[i].ToSampleProvider();
            }

            var concatenating = new ConcatenatingSampleProvider(providers);
            mixer = new MusicMixer(song,concatenating, readers);
        }
        else
        {
            var reader = new MediaFoundationReader(song[0].Url);
            mixer = new MusicMixer(reader.ToSampleProvider(), [reader]);
        }

        _queue.Enqueue(mixer);
        if (_queue.Count == 1)
        {
            LoadNextTrack();
        }
    }

    public void EnqueueTrack(string uri)
    {
        var reader = new MediaFoundationReader(uri);
        var mixer = new MusicMixer(reader.ToSampleProvider(), [reader]); // only wave
        _queue.Enqueue(mixer);
        if (_queue.Count == 1 && CurrentReader == null)
        {
            LoadNextTrack();
        }
    }

    public void LoadNextTrack()
    {
        try
        {
            if (_queue.Count > 0)
            {
                if (CurrentReader != null)
                {
                    CurrentReader.Dispose();
                }
                var nextTrack = _queue.Dequeue();
                if (nextTrack.IsMixed || _jointlessTimes == null)
                {
                    _jointlessTimes = new();
                    for (int i = 0; i < nextTrack.Music.Length; i++)
                    {
                        var music = nextTrack.Music[i];
                        TimeSpan startTime;
                        if (i == 0)
                        {
                            startTime = TimeSpan.Zero;
                        }
                        else
                        {
                            startTime = _jointlessTimes[^1].Item3;
                        }

                        _jointlessTimes.Add(
                            new Tuple<IMusic, TimeSpan, TimeSpan, MediaFoundationReader>
                                (music, startTime, startTime + music.Duration, nextTrack.Reader[i])
                        );
                    }

                    _currentJointlessTime = _jointlessTimes[0];
                    IsMixed = true;
                }
                else
                {
                    if (nextTrack.Music != null) CurrentMusic = nextTrack.Music[0];
                }
                
                OutputDevice.Stop();
                CurrentMusic = nextTrack.Music[0];
                CurrentReader = nextTrack.Reader[0];
                VolumeProvider = new VolumeSampleProvider(nextTrack.Wave)
                {
                    Volume = _volume
                };
                OutputDevice.Init(VolumeProvider);
                OutputDevice.Play();
                MusicChangedHandle?.Invoke(CurrentMusic);
            }
        }
        catch (Exception e)
        {
            ExceptionService.Instance.Throw(e);
        }
    }

    private void GetCurrentInfo(object? sender, ElapsedEventArgs e)
    {
        if (OutputDevice != null && PlayStateChangedHandle != null)
        {
            if (CurrentReader != null)
            {
                // 无缝切换部分
                if (IsMixed)
                {
                    if (_currentJointlessTime != null && _currentJointlessTime.Item3 <= Position)
                    {
                        foreach (var item in _jointlessTimes)
                        {
                            if (item.Item2 <= Position && item.Item3 > Position)
                            {
                                _currentJointlessTime = item;
                                if (CurrentMusic != item.Item1)
                                {
                                    CurrentMusic = item.Item1;
                                    MusicChangedHandle?.Invoke(CurrentMusic);
                                }

                                CurrentReader = item.Item4;
                                break;
                            }
                        }
                    }
                }
                else if (CurrentMusic != null && CurrentReader.Position >= CurrentReader.Length)
                {
                    LoadNextTrack();
                }

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

    public class MusicMixer
    {
        public readonly MediaFoundationReader[]? Reader;
        public readonly ISampleProvider Wave;
        public readonly IMusic[]? Music;
        public bool HasMusic => Music != null;
        public bool IsMixed;

        public MusicMixer(ISampleProvider wave, MediaFoundationReader[] reader)
        {
            Wave = wave;
            IsMixed = false;
            Music = null;
            Reader = reader;
        }

        public MusicMixer(IMusic[] music, ISampleProvider wave, MediaFoundationReader[] reader)
        {
            Music = music;
            Wave = wave;
            if (music.Length > 1)
            {
                IsMixed = true;
            }

            Reader = reader;
        }
    }

    public void Dispose()
    {
        OutputDevice?.Dispose();
        CurrentReader?.Dispose();
    }
}