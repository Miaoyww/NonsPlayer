using System.Windows.Input;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using NonsPlayer.Models;
using NonsPlayer.ViewModels;

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
        ServiceHelper.DispatcherQueue.TryEnqueue(() => { MusicStateModel.Instance.IsPlaying = isPlaying; });
    }

    public void OnPositionChanged(TimeSpan position)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            if (MusicStateModel.Instance.OnDrag) return;
            MusicStateModel.Instance.Position = position.TotalSeconds;
        });
    }

    public void OnMusicChanged(Music music)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() => { MusicStateModel.Instance.CurrentMusic = music; });
    }

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

    private void OnPreviousMusic()
    {
        PlayQueue.Instance.PlayPrevious(true);
    }

    private void OnNextMusic()
    {
        PlayQueue.Instance.PlayNext(true);
    }
}