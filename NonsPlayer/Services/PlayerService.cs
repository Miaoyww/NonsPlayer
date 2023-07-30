using System.Windows.Input;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;
using NonsPlayer.Models;
using NonsPlayer.Helpers;

namespace NonsPlayer.Services;

public class PlayerService
{
    public static PlayerService Instance
    {
        get;
    } = new();

    private PlayerService()
    {
        MusicPlayCommand = new RelayCommand(() =>
        {
            Player.Instance.Play();
        });
        VolumeMuteCommand = new RelayCommand(Mute);
        NextMusicCommand = new RelayCommand(OnNextMusic);
        PreviousMusicCommand = new RelayCommand(OnPreviousMusic);
        Player.Instance.PlayStateChangedHandle += OnPlaystateChanged;
        Player.Instance.MusicChangedHandle += OnMusicChanged;
        Player.Instance.PositionChangedHandle += OnPositionChanged;
    }
    
    public void OnPlaystateChanged(bool isPlaying)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            MusicState.Instance.IsPlaying = isPlaying;
        });
    }

    public void OnPositionChanged(TimeSpan position)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            MusicState.Instance.Position = position;
        });
    }

    public void OnMusicChanged(Music music)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            MusicState.Instance.CurrentMusic = music;
        });
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

    private void OnPreviousMusic() => PlayQueue.Instance.PlayPrevious(isUserPressed: true);

    private void OnNextMusic() => PlayQueue.Instance.PlayNext(isUserPressed: true);

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
}

