using NAudio.Wave;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Nons.Player;

public class PlayQueue
{
    public enum PlayModeEnum
    {
        Sequential, //顺序播放
        Random, //随机播放
        SingleLoop, //单曲循环
        ListLoop //列表循环
    }

    private Music _currentMusic;
    private bool _isUserPressed;
    private List<Music> _randomMusicList = new();

    public PlayQueue()
    {
        MusicList = new List<Music>();
        PlayMode = PlayModeEnum.ListLoop;
        Player.Instance.OutputDevice.PlaybackStopped += OnMusicStopped;
        CurrentMusicChanged += OnCurrentMusicChanged;
    }

    public static PlayQueue Instance { get; } = new();

    public List<Music> MusicList { get; set; }
    public int Count => MusicList.Count;

    public Music CurrentMusic
    {
        get => _currentMusic;
        set
        {
            if (value == _currentMusic) return;

            _currentMusic = value;
            CurrentMusicChanged(value);
        }
    }


    public PlayModeEnum PlayMode { get; set; }

    public delegate void MusicAddedEventHandler(Music value);

    public delegate void CurrentMusicChangedEventHandler(Music value);

    public delegate void PlaylistAddedEventHandler();

    public event CurrentMusicChangedEventHandler CurrentMusicChanged;
    public event MusicAddedEventHandler MusicAdded;
    public event PlaylistAddedEventHandler PlaylistAdded;

    public void OnMusicStopped(object sender, StoppedEventArgs e)
    {
        if (PlayMode == PlayModeEnum.ListLoop && Player.Instance.IsInitializingNewMusic == false)
            if (Player.Instance.CurrentMusic != Player.Instance.PreviousMusic && PlayMode != PlayModeEnum.SingleLoop)
            {
                if (_isUserPressed) return;
                PlayNext();
            }
    }

    // 当CurrentMusic改变时，将触发PlayerService的NewPlay方法
    public void OnCurrentMusicChanged(Music value)
    {
        Player.Instance.NewPlay(value);
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
        if (PlayMode is PlayModeEnum.Random) _randomMusicList = GetRandomList(CurrentMusic);
    }

    public void AddMusicList(Music[] musicList)
    {
        Clear();
        MusicList.AddRange(musicList);
        Play(musicList[0], true);
        PlaylistAdded();
    }

    public void AddMusic(Music music)
    {
        if (MusicList.Contains(music))
        {
            return;
        }

        if (MusicList.Count == 0)
        {
            MusicList.Add(music);
            MusicAdded(music);
            return;
        }

        MusicList.Insert(MusicList.IndexOf(CurrentMusic) + 1, music);
        MusicAdded(music);
        if (PlayMode is PlayModeEnum.Random) _randomMusicList.Insert(MusicList.IndexOf(CurrentMusic) + 1, music);
    }

    public void RemoveMusic(Music music)
    {
        MusicList.Remove(music);
        if (PlayMode is PlayModeEnum.Random) _randomMusicList.Remove(music);
    }

    public void Clear()
    {
        MusicList.Clear();
        _randomMusicList.Clear();
    }

    public void Play(Music music, bool rePlay = false)
    {
        if (music == CurrentMusic)
        {
            Player.Instance.Play(rePlay);
            return;
        }

        if (!MusicList.Contains(music))
        {
            AddMusic(music);
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
            if (index >= list.Count) return;

            Play(list[index]);
            return;
        }

        if (PlayMode is PlayModeEnum.ListLoop)
        {
            if (list.Count == 0) return;

            index = index >= list.Count ? 0 : index;
            Play(list[index]);
        }
    }

    public void PlayPrevious(bool isUserPressed = false)
    {
        _isUserPressed = isUserPressed;
        var list = PlayMode is PlayModeEnum.Random ? _randomMusicList : MusicList;
        var index = list.IndexOf(CurrentMusic) - 1;
        if (index < 0) index = list.Count - 1;

        Play(list[index]);
    }

    public int GetIndex(Music music)
    {
        return MusicList.IndexOf(music);
    }

    /// <summary>
    ///     获取随机播放列表
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