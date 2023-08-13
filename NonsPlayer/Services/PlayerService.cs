using System.Windows.Input;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using NonsPlayer.Models;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Services;

public partial class PlayerService
{
    private PlayerService()
    {
        Player.Instance.PlayStateChangedHandle += OnPlaystateChanged;
        Player.Instance.MusicChangedHandle += OnMusicChanged;
        Player.Instance.PositionChangedHandle += OnPositionChanged;
    }

    public static PlayerService Instance { get; } = new();

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
    public void Play() => Player.Instance.Play();

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
    private void PreviousMusic() => PlayQueue.Instance.PlayPrevious(true);

    [RelayCommand]
    private void NextMusic() => PlayQueue.Instance.PlayNext(true);
}