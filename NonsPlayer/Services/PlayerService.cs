﻿using System.Windows.Input;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;
using NonsPlayer.Helpers;
using NonsPlayer.Models;

namespace NonsPlayer.Services;

public class PlayerService
{
    private PlayerService()
    {
        MusicPlayCommand = new RelayCommand(() => { Player.Instance.Play(); });
        VolumeMuteCommand = new RelayCommand(Mute);
        NextMusicCommand = new RelayCommand(OnNextMusic);
        PreviousMusicCommand = new RelayCommand(OnPreviousMusic);
        Player.Instance.PlayStateChangedHandle += OnPlaystateChanged;
        Player.Instance.MusicChangedHandle += OnMusicChanged;
        Player.Instance.PositionChangedHandle += OnPositionChanged;
    }

    public static PlayerService Instance { get; } = new();

    public ICommand MusicPlayCommand { get; }

    public ICommand NextMusicCommand { get; }

    public ICommand PreviousMusicCommand { get; }

    public ICommand VolumeMuteCommand { get; }

    public void OnPlaystateChanged(bool isPlaying)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() => { MusicState.Instance.IsPlaying = isPlaying; });
    }

    public void OnPositionChanged(TimeSpan position)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            if (MusicState.Instance.OnDrag) return;
            MusicState.Instance.Position = position.TotalSeconds;
        });
    }

    public void OnMusicChanged(Music music)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() => { MusicState.Instance.CurrentMusic = music; });
    }

    private void Mute()
    {
        if (MusicState.Instance.Volume > 0)
        {
            MusicState.Instance.PreviousVolume = MusicState.Instance.Volume;
            MusicState.Instance.Volume = 0;
        }
        else
        {
            MusicState.Instance.Volume = MusicState.Instance.PreviousVolume;
        }
    }

    private void OnPreviousMusic()
    {
        PlayQueue.Instance.PlayPrevious(true);
    }

    private void OnNextMusic()
    {
        PlayQueue.Instance.PlayNext(true);
    }
}