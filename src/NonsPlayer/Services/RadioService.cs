using System.Reflection;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.Extensions.Logging;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;

namespace NonsPlayer.Services;

public partial class RadioService : ObservableObject
{
    #region 事件注册

    public delegate void RadioMusicChangedEventHandler(IMusic music);

    public event RadioMusicChangedEventHandler? RadioMusicChanged;

    #endregion

    public IAdapter CurrentAdapter;
    public List<IMusic> RadioSongs = new();
    public bool IsStarted = false;
    private ILogger logger = App.GetLogger<RadioService>();

    public void Start(IMusic[] music, IAdapter adapter)
    {
        logger.LogInformation($"RadioService started, available adapter: {adapter.GetMetadata().DisplayPlatform}");
        CurrentAdapter = adapter;
        RadioSongs.AddRange(music);
        PlayQueue.Instance.AddMusicList(music);
        PlayQueue.Instance.Play(music[0]);
        PlayQueue.Instance.IsRadioMode = true;
        PlayQueue.Instance.RadioWaiting += OnRadioWaiting;
        PlayQueue.Instance.MusicAdded += PlayQueueOnMusicAdded;
        PlayQueue.Instance.CurrentMusicChanged += PlayQueueCurrentMusicChanged;
        IsStarted = true;
    }

    private void PlayQueueCurrentMusicChanged(IMusic value)
    {
        if (RadioSongs.Contains(value))
        {
            RadioMusicChanged?.Invoke(value);
        }
    }

    public IMusic? GetCurrentMusic()
    {
        if (IsStarted)
        {
            return PlayQueue.Instance.CurrentMusic;
        }
        else
        {
            return null;
        }
    }

    public void Stop()
    {
        logger.LogInformation("RadioService stopped");
        PlayQueue.Instance.IsRadioMode = false;
        PlayQueue.Instance.RadioWaiting -= OnRadioWaiting;
        PlayQueue.Instance.MusicAdded -= PlayQueueOnMusicAdded;
        IsStarted = false;
    }

    private async void OnRadioWaiting()
    {
        logger.LogInformation("RadioService now on waiting");
        PlayQueue.Instance.AddMusicList(await GetRadioSong());
        PlayQueue.Instance.PlayNext();
    }


    private void PlayQueueOnMusicAdded(IMusic value)
    {
        if (!RadioSongs.Contains(value))
        {
            Stop();
        }
    }

    public async Task<IMusic[]> GetRadioSong()
    {
        logger.LogInformation("GetRadioSong Handled");
        var songs = await CurrentAdapter.Common.GetRadioSong();
        RadioSongs.AddRange(songs);
        return songs;
    }
}