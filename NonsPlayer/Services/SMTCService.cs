using System.Timers;
using Windows.Media;
using Windows.Media.Playback;
using Windows.Storage.Streams;
using NAudio.Wave;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Services;

public class SMTCService
{
    private MediaPlayer _player = new();
    private SystemMediaTransportControls _smtc;
    private SMTCUpdater _updater;
    private IMusic _currentMusic;

    public SMTCService()
    {
        Player.Instance.MusicChangedHandle += MusicChangedHandle;

        _player.CommandManager.IsEnabled = false;
        //直接创建SystemMediaTransportControls对象被平台限制，神奇的是MediaPlayer对象可以创建该NativeObject
        _smtc = _player.SystemMediaTransportControls;
        _updater = new SMTCUpdater(_smtc.DisplayUpdater, "NonsPlayer");

        //启用状态
        _smtc.IsEnabled = true;
        _smtc.IsPlayEnabled = true;
        _smtc.IsPauseEnabled = true;
        _smtc.IsNextEnabled = true;
        _smtc.IsPreviousEnabled = true;
        //响应系统播放器的命令
        _smtc.ButtonPressed += SmtcOnButtonPressed;
    }

    public void Dispose()
    {
        _smtc.IsEnabled = false;
    }

    private async void SmtcOnButtonPressed(SystemMediaTransportControls sender,
        SystemMediaTransportControlsButtonPressedEventArgs args)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(async () =>
        {
            switch (args.Button)
            {
                case SystemMediaTransportControlsButton.Play:
                    await Player.Instance.Play();
                    break;
                case SystemMediaTransportControlsButton.Pause:
                    await Player.Instance.Play();
                    break;
                case SystemMediaTransportControlsButton.Next:
                    PlayQueue.Instance.PlayNext();
                    break;
                case SystemMediaTransportControlsButton.Previous:
                    PlayQueue.Instance.PlayPrevious();
                    break;
            }
        });
    }

    private void MusicChangedHandle(IMusic currentmusic)
    {
        _currentMusic = currentmusic;
        if (_currentMusic == null)
        {
            return;
        }

        _smtc.PlaybackStatus = Player.Instance.OutputDevice.PlaybackState == PlaybackState.Playing
            ? MediaPlaybackStatus.Playing
            : MediaPlaybackStatus.Paused;
        _updater.SetArtist(_currentMusic.ArtistsName)
            .SetAlbumTitle(_currentMusic.AlbumName)
            .SetTitle(_currentMusic.Name)
            .SetThumbnail(_currentMusic.GetCoverUrl())
            .Update();
    }

    public void Init()
    {
    }
}

public class SMTCUpdater
{
    private readonly SystemMediaTransportControlsDisplayUpdater _updater;

    public SMTCUpdater(SystemMediaTransportControlsDisplayUpdater Updater, string AppMediaId)
    {
        _updater = Updater;
        _updater.AppMediaId = AppMediaId;
        _updater.Type = MediaPlaybackType.Music;
    }

    public SMTCUpdater SetTitle(string title)
    {
        _updater.MusicProperties.Title = title;
        return this;
    }

    public SMTCUpdater SetAlbumTitle(string albumTitle)
    {
        _updater.MusicProperties.AlbumTitle = albumTitle;
        return this;
    }

    public SMTCUpdater SetArtist(string artist)
    {
        _updater.MusicProperties.Artist = artist;
        return this;
    }

    public SMTCUpdater SetThumbnail(string ImgUrl)
    {
        _updater.Thumbnail = RandomAccessStreamReference.CreateFromUri(new Uri(ImgUrl));
        return this;
    }

    public void Update() => _updater.Update();
}