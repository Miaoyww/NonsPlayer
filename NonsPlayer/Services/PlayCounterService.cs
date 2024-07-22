using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Nons.Player;

namespace NonsPlayer.Core.Services;

public class PlayCounterService
{
    public List<IMusic> RecentlyMusic = new();
    public Dictionary<IMusic, int> Data;
    public int TotalPlayCount => Data.Values.Sum();

    public delegate void CounterChangedHandler();

    public event CounterChangedHandler CounterChanged;

    public void Init()
    {
        Data = new();
        Player.Instance.MusicChangedHandle += OnMusicChanged;
        RecentlyMusic = new List<IMusic>();
    }

    private void OnMusicChanged(IMusic music)
    {
        RecordNewPlay(music);
        CounterChanged?.Invoke();
    }

    public void RecordNewPlay(IMusic music)
    {
        if (!Data.ContainsKey(music))
        {
            Data.Add(music, 1);
        }
        else
        {
            Data[music] += 1;
        }

        // Check if the music already exists in RecentlyMusic list
        var existingMusic = RecentlyMusic.FirstOrDefault(m => m.Id == music.Id);
        if (existingMusic != null)
        {
            RecentlyMusic.Remove(existingMusic);
        }

        RecentlyMusic.Add(music);
    }
}