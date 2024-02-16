using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Services;

public partial class PlayerService : ObservableRecipient
{
    [ObservableProperty] private PlayQueue.PlayModeEnum currentPlayMode;
    [ObservableProperty] private bool isShuffle;

    private PlayerService()
    {
        PlayQueue.Instance.PlayModeChanged += OnPlayModeChanged;
        PlayQueue.Instance.ShuffleChanged += OnShuffleChanged;
        Player.Instance.PlayStateChangedHandle += OnPlaystateChanged;
        Player.Instance.MusicChangedHandle += OnMusicChanged;
        Player.Instance.PositionChangedHandle += OnPositionChanged;
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

    public void OnPositionChanged(TimeSpan position)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            // if (MusicStateModel.Instance.OnDrag) return;
            MusicStateModel.Instance.Position = position.TotalSeconds;
        });
    }

    public void OnMusicChanged(Music music)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() => { MusicStateModel.Instance.CurrentMusic = music; });
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
        PlayQueue.Instance.PlayPrevious(true);
    }

    [RelayCommand]
    private void NextMusic()
    {
        PlayQueue.Instance.PlayNext(true);
    }
}