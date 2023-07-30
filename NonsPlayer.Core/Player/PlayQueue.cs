using NAudio.Wave;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Player;

public class PlayQueue
{
    public static PlayQueue Instance
    {
        get;
    } = new();

    private Music _currentMusic;
    private List<Music> _randomMusicList = new();
    private bool _isUserPressed = false;

    public event EventHandler MusicAddedEventHandler;
    public event EventHandler PlaylistAddedEventHandler;

    public List<Music> MusicList
    {
        get;
        set;
    }

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

    /// <summary>
    /// 模式选择器
    /// </summary>
    public PlayModeEnum PlayMode
    {
        get;
        set;
    }

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

    /// <summary>
    /// 改变模式
    /// </summary>
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

    /// <summary>
    /// 添加歌单到播放列表
    /// </summary>
    /// <param name="musicList">歌曲</param>
    public void AddMusicList(List<Music> musicList)
    {
        Clear();
        MusicList.AddRange(musicList);
        Play(musicList[0]);
        PlaylistAddedEventHandler?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>
    /// 添加歌曲到播放列表
    /// </summary>
    /// <param name="music">待添加的歌曲</param>
    public void AddMusic(Music music)
    {
        // 若播放列表为空，那么直接添加到播放列表中
        if (MusicList.Count == 0)
        {
            MusicList.Add(music);
            CurrentMusic = music;
            MusicAddedEventHandler?.Invoke(this, new MusicAddedEventArgs(music, 0));
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
        MusicAddedEventHandler?.Invoke(this, new MusicAddedEventArgs(music, MusicList.IndexOf(CurrentMusic) + 1));
        if (PlayMode is PlayModeEnum.Random)
        {
            // 直接在随机播放列表的当前播放音乐后插入
            _randomMusicList.Insert(MusicList.IndexOf(CurrentMusic) + 1, music);
        }

        CurrentMusic = music;
    }

    /// <summary>
    /// 删除指定歌曲
    /// </summary>
    /// <param name="music">待删除的歌曲</param>
    public void RemoveMusic(Music music)
    {
        MusicList.Remove(music);
        if (PlayMode is PlayModeEnum.Random)
        {
            _randomMusicList.Remove(music);
        }
    }

    /// <summary>
    /// 清空播放列表
    /// </summary>
    public void Clear()
    {
        MusicList.Clear();
        _randomMusicList.Clear();
    }

    /// <summary>
    /// 播放歌曲
    /// </summary>
    public void Play(Music music)
    {
        // 若当前音乐正在播放，则触发Play，正在播放则暂停，否则播放
        if (music == CurrentMusic)
        {
            Player.Instance.Play();
            return;
        }

        CurrentMusic = music;
        _isUserPressed = false;
    }

    /// <summary>
    /// 播放下一首歌曲
    /// </summary>
    public void PlayNext(bool isUserPressed = false)
    {
        _isUserPressed = isUserPressed;
        var list = PlayMode is PlayModeEnum.Random ? _randomMusicList : MusicList;

        // 如果是单曲循环，那么就播放当前歌曲
        if (PlayMode is PlayModeEnum.SingleLoop)
        {
            Play(CurrentMusic);
            return;
        }

        // 如果是顺序播放，那么到了最后一首歌就停止播放
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

        // 如果是列表循环，那么到了最后一首歌就播放第一首歌
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

    /// <summary>
    /// 播放上一首歌曲
    /// </summary>
    public void PlayPrevious(bool isUserPressed = false)
    {
        _isUserPressed = isUserPressed;
        // 若当前播放模式为随机播放，则获取随机播放列表
        var list = PlayMode is PlayModeEnum.Random ? _randomMusicList : MusicList;
        // 获取上一首歌曲
        var index = list.IndexOf(CurrentMusic) - 1;
        if (index < 0)
        {
            index = list.Count - 1;
        }

        Play(list[index]);
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

public class MusicAddedEventArgs : EventArgs
{
    public Music Music
    {
        get;
    }

    public int Index
    {
        get;
    }

    public MusicAddedEventArgs(Music music, int index)
    {
        Music = music;
        Index = index;
    }
}