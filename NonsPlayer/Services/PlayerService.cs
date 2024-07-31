using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.Extensions.Logging;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using Serilog.Core;

namespace NonsPlayer.Services;

public partial class PlayerService : ObservableRecipient
{
    [ObservableProperty] private PlayQueue.PlayModeEnum currentPlayMode;
    [ObservableProperty] private bool isShuffle;
    public static List<TimeSpan> TargetSeekingTimeSpans = new List<TimeSpan>();
    public static int IntervalCounter = 0;
    private static Task _playerLoaderTask;
    public static bool LockSeeking = false;
    public static int CurrentRunningSeekingHandler = 0;

    private ILogger logger = App.GetLogger<PlayerService>();
    public static TimeSpan RunningTimeSpan = TimeSpan.Zero;

    private PlayerService()
    {
        PlayQueue.Instance.PlayModeChanged += OnPlayModeChanged;
        PlayQueue.Instance.ShuffleChanged += OnShuffleChanged;
        Player.Instance.PlayStateChangedHandle += OnPlaystateChanged;
        Player.Instance.MusicChangedHandle += OnMusicChanged;
        CurrentPlayMode = PlayQueue.PlayModeEnum.ListLoop; //TODO: 播放状态储存
    }

    public static PlayerService Instance { get; } = new();

    private void OnPlayModeChanged(PlayQueue.PlayModeEnum mode)
    {
        CurrentPlayMode = mode;
    }

    private void OnShuffleChanged(bool value)
    {
        IsShuffle = value;
    }

    public void OnPlaystateChanged(bool isPlaying)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() => { MusicStateModel.Instance.IsPlaying = isPlaying; });
    }


    public void OnMusicChanged(IMusic music)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            MusicStateModel.Instance.CurrentMusic = music;
        });
    }

    [RelayCommand]
    public void Play()
    {
        Player.Instance.Play();
    }

    [RelayCommand]
    private void Mute()
    {
        if (MusicStateModel.Instance.Volume > 0)
        {
            MusicStateModel.Instance.PreviousVolume = MusicStateModel.Instance.Volume;
            MusicStateModel.Instance.Volume = 0;
        }
        else
        {
            MusicStateModel.Instance.Volume = MusicStateModel.Instance.PreviousVolume;
        }
    }

    [RelayCommand]
    private void PreviousMusic()
    {
        logger.LogInformation("Play previous music");

        PlayQueue.Instance.PlayPrevious(true);
    }

    [RelayCommand]
    private void NextMusic()
    {
        logger.LogInformation("Play next music");
        PlayQueue.Instance.PlayNext(true);
    }

    private static void PlaybackSession_SeekCompleted()
    {
        DecreaseRunningSeekingHandler();
    }

    public static async void DecreaseRunningSeekingHandler()
    {
        TargetSeekingTimeSpans.Clear();
    }

    public static void Seek(TimeSpan? targetTimeSpan, bool isHandler = false)
    {
        lock (TargetSeekingTimeSpans)
        {
            if (isHandler)
            {
                var timespan = TargetSeekingTimeSpans.Last();
                RunningTimeSpan = timespan;
                Player.Instance.SetPosition(timespan);
                IntervalCounter++;
            }
            else
            {
                if (_playerLoaderTask != null && _playerLoaderTask.IsCompleted == false) return;
                if (TargetSeekingTimeSpans.Count == 0 && !LockSeeking)
                {
                    TargetSeekingTimeSpans.Add(targetTimeSpan.Value);
                    RunningTimeSpan = targetTimeSpan.Value;
                    Player.Instance.SetPosition(targetTimeSpan.Value);
                    IntervalCounter++;
                }
                else
                {
                    TargetSeekingTimeSpans.Add(targetTimeSpan.Value);
                }
            }
        }

        PlaybackSession_SeekCompleted();
    }
}