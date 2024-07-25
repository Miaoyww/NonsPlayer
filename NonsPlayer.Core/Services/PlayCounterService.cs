using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Nons.Player;

namespace NonsPlayer.Core.Services;

public class PlayCounterService
{
    public List<IMusic> RecentlyMusic = new();
    public Dictionary<IMusic, int> Data;

    public int TotalPlayCount;
    public TimeSpan TodayPlayDuration;

    public delegate void CounterChangedHandler();

    public event CounterChangedHandler CounterChanged;

    public void Init(int totalPlayCount, Tuple<DateTime, TimeSpan> todayPlayDuration)
    {
        Data = new();
        Player.Instance.MusicChangedHandle += OnMusicChanged;
        Player.Instance.PositionChangedHandle += PositionChangedHandle;
        RecentlyMusic = new List<IMusic>();
        TotalPlayCount = totalPlayCount;
        if (todayPlayDuration != null)
        {
            if (todayPlayDuration.Item1 != DateTime.Now.Date)
            {
                TodayPlayDuration = new TimeSpan(0);
            }
            else
            {
                TodayPlayDuration = todayPlayDuration.Item2;
            }
        }
        else
        {
            TodayPlayDuration = new TimeSpan(0);
        }
    }

    private void PositionChangedHandle(TimeSpan time)
    {
        TodayPlayDuration += TimeSpan.FromMilliseconds(10);
    }

    private void OnMusicChanged(IMusic music)
    {
        RecordNewPlay(music);
        CounterChanged?.Invoke();
        ConfigManager.Instance.Settings.TodayPlayDuration =
            new Tuple<DateTime, TimeSpan>(DateTime.Now.Date, TodayPlayDuration);
        ConfigManager.Instance.Save();
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
        TotalPlayCount += 1;
        ConfigManager.Instance.Settings.TotalPlayCount = TotalPlayCount;
    }
}