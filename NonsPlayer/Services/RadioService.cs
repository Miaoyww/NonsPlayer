using System.Reflection;
using CommunityToolkit.Mvvm.ComponentModel;
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

    public void Start(IMusic[] music, IAdapter adapter)
    {
        CurrentAdapter = adapter;
        RadioSongs.AddRange(music);
        PlayQueue.Instance.AddMusicList(music);
        PlayQueue.Instance.Play(music[0]);
        PlayQueue.Instance.IsRadioMode = true;
        PlayQueue.Instance.RadioWatting += OnRadioWatting;
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
        PlayQueue.Instance.IsRadioMode = false;
        PlayQueue.Instance.RadioWatting -= OnRadioWatting;
        PlayQueue.Instance.MusicAdded -= PlayQueueOnMusicAdded;
        IsStarted = false;
    }

    private async void OnRadioWatting()
    {
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
        var songs = await CurrentAdapter.Common.GetRadioSong();
        RadioSongs.AddRange(songs);
        return songs;
    }
}