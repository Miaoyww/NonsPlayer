using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Nons.Player;

namespace NonsPlayer.Core.Services;

public class PlayCounter
{
    public Dictionary<IMusic, int> Data;

    public PlayCounter()
    {
        Init();
        Player.Instance.MusicChangedHandle += OnMusicChanged;
    }

    private void OnMusicChanged(IMusic music)
    {
        RecordNewPlay(music);
    }

    public void RecordNewPlay(IMusic music)
    {
        Data[music] += 1;
    }

    private void Init()
    {
        //TODO: 完善播放计数器
    }
}