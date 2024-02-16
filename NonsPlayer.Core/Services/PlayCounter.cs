using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;


namespace NonsPlayer.Core.Services;

public class PlayCounter
{
    public Dictionary<Music, int> Data;

    public PlayCounter()
    {
        Init();
        Player.Instance.MusicChangedHandle += OnMusicChanged;
    }

    private void OnMusicChanged(Music music)
    {
        RecordNewPlay(music);
    }

    public void RecordNewPlay(Music music)
    {
        Data[music] += 1;
    }

    private void Init()
    {
        //TODO: 完善播放计数器
    }
}