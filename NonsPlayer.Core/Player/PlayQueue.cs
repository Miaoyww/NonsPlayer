using NAudio.Wave;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Player;

public class PlayQueue
{
    public static PlayQueue Instance { get; } = new();

    private Music _currentMusic;
    private List<Music> _randomMusicList = new();
    private bool _isUserPressed = false;

    public delegate void MusicAddedEventHandler(Music value);

    public delegate void PlaylistAddedEventHandler();

    public event MusicAddedEventHandler MusicAdded;
    public event PlaylistAddedEventHandler PlaylistAdded;

    public List<Music> MusicList { get; set; }
    public int Count => MusicList.Count;

    public Music CurrentMusic
    {
        get => _currentMusic;
        set
        {
            if (value == _currentMusic)
            {
                return;
            }

            _currentMusic = value;
            OnCurrentMusicChanged();
        }
    }

    public enum PlayModeEnum
    {
        Sequential = 0, //顺序播放
        Random = 1, //随机播放
        SingleLoop = 2, //单曲循环
        ListLoop = 3, //列表循环
    }


    public PlayModeEnum PlayMode { get; set; }

    public PlayQueue()
    {
        MusicList = new();
        PlayMode = PlayModeEnum.ListLoop;
        Player.Instance.OutputDevice.PlaybackStopped += OnMusicStopped;
    }

    public void OnMusicStopped(object sender, StoppedEventArgs e)
    {
        if (PlayMode == PlayModeEnum.ListLoop && Player.Instance.IsInitializingNewMusic == false)
        {
            if (Player.Instance.CurrentMusic != Player.Instance.PreviousMusic && PlayMode != PlayModeEnum.SingleLoop)
            {
                if (_isUserPressed)
                {
                    return;
                }

                PlayNext();
            }
        }
    }

    // 当CurrentMusic改变时，将触发PlayerService的NewPlay方法
    public void OnCurrentMusicChanged()
    {
        Player.Instance.NewPlay(CurrentMusic);
    }

    public void ChangePlayMode()
    {
        PlayMode = PlayMode switch
        {
            PlayModeEnum.Sequential => PlayModeEnum.Random,
            PlayModeEnum.Random => PlayModeEnum.SingleLoop,
            PlayModeEnum.SingleLoop => PlayModeEnum.ListLoop,
            PlayModeEnum.ListLoop => PlayModeEnum.Sequential,
            _ => PlayModeEnum.Sequential
        };
        // 如果模式是随机播放，调用GetRandomList方法
        if (PlayMode is PlayModeEnum.Random)
        {
            _randomMusicList = GetRandomList(CurrentMusic);
        }
    }

    public void AddMusicList(Music[] musicList)
    {
        Clear();
        MusicList.AddRange(musicList);
        Play(musicList[0]);
        PlaylistAdded();
    }

    public void AddMusic(Music music)
    {
        // 若播放列表为空，那么直接添加到播放列表中
        if (MusicList.Count == 0)
        {
            MusicList.Add(music);
            CurrentMusic = music;
            MusicAdded(music);
            return;
        }

        // 如果播放列表中已存在该歌曲，那么直接播放该歌曲
        if (MusicList.Contains(music))
        {
            CurrentMusic = music;
            return;
        }

        // 如果播放列表不为空，那么就在当前播放歌曲的后面插入
        MusicList.Insert(MusicList.IndexOf(CurrentMusic) + 1, music);
        MusicAdded(music);
        if (PlayMode is PlayModeEnum.Random)
        {
            _randomMusicList.Insert(MusicList.IndexOf(CurrentMusic) + 1, music);
        }

        CurrentMusic = music;
    }

    public void RemoveMusic(Music music)
    {
        MusicList.Remove(music);
        if (PlayMode is PlayModeEnum.Random)
        {
            _randomMusicList.Remove(music);
        }
    }

    public void Clear()
    {
        MusicList.Clear();
        _randomMusicList.Clear();
    }

    public void Play(Music music)
    {
        if (music == CurrentMusic)
        {
            Player.Instance.Play();
            return;
        }

        CurrentMusic = music;
        _isUserPressed = false;
    }

    public void PlayNext(bool isUserPressed = false)
    {
        _isUserPressed = isUserPressed;
        var list = PlayMode is PlayModeEnum.Random ? _randomMusicList : MusicList;
        if (PlayMode is PlayModeEnum.SingleLoop)
        {
            Play(CurrentMusic);
            return;
        }

        var index = list.IndexOf(CurrentMusic) + 1;
        if (PlayMode is PlayModeEnum.Sequential)
        {
            if (index >= list.Count)
            {
                return;
            }

            Play(list[index]);
            return;
        }

        if (PlayMode is PlayModeEnum.ListLoop)
        {
            if (list.Count == 0)
            {
                return;
            }

            index = index >= list.Count ? 0 : index;
            Play(list[index]);
        }
    }

    public void PlayPrevious(bool isUserPressed = false)
    {
        _isUserPressed = isUserPressed;
        var list = PlayMode is PlayModeEnum.Random ? _randomMusicList : MusicList;
        var index = list.IndexOf(CurrentMusic) - 1;
        if (index < 0)
        {
            index = list.Count - 1;
        }

        Play(list[index]);
    }

    public int GetIndex(Music music)
    {
        return MusicList.IndexOf(music);
    }

    /// <summary>
    /// 获取随机播放列表
    /// </summary>
    /// <param name="music">当前播放的歌曲</param>
    private List<Music> GetRandomList(Music music)
    {
        var random = new Random();
        var list = MusicList.ToList();
        list.RemoveAt(0);
        list = list.OrderBy(x => random.Next()).ToList();
        list.Insert(0, music);
        return list;
    }
}