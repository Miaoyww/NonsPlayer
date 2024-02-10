using NAudio.Wave;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Nons.Player;

public class PlayQueue
{
    public delegate void PlayQueueChangedHandler();
    public delegate void CurrentMusicChangedEventHandler(Music value);

    public delegate void MusicAddedEventHandler(Music value);

    public delegate void PlaylistAddedEventHandler();

    public enum PlayModeEnum
    {
        Sequential, //顺序播放
        Random, //随机播放
        SingleLoop, //单曲循环
        ListLoop, //列表循环
        Recommend
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
        MusicAdded += OnMusicAdded;
        PlaylistAdded += OnPlaylistAdded;
    }

    private void OnPlaylistAdded()
    {
        CurrentQueueChanged?.Invoke();
    }

    private void OnMusicAdded(Music value)
    {
        CurrentQueueChanged?.Invoke();
        
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

    public event CurrentMusicChangedEventHandler CurrentMusicChanged;
    public event PlayQueueChangedHandler CurrentQueueChanged;
    public event MusicAddedEventHandler MusicAdded;
    public event PlaylistAddedEventHandler PlaylistAdded;

    public void OnMusicStopped(object sender, StoppedEventArgs e)
    {
        if (PlayMode == PlayModeEnum.ListLoop)
            if (Player.Instance.CurrentMusic != Player.Instance.PreviousMusic && PlayMode != PlayModeEnum.SingleLoop)
            {
                if (_isUserPressed) return;
                if (CurrentMusic.TotalTime.TotalSeconds - Player.Instance.Position.TotalSeconds > 1) return;
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
            PlayModeEnum.ListLoop => PlayModeEnum.Recommend,
            PlayModeEnum.Recommend => PlayModeEnum.Sequential,
            _ => PlayModeEnum.Sequential
        };
        // 如果模式是随机播放，调用GetRandomList方法
        if (PlayMode is PlayModeEnum.Random) _randomMusicList = GetRandomList(CurrentMusic);
    }

    public void AddMusicList(Music[] musicList)
    {
        Clear();
        MusicList.AddRange(musicList);
        Play(musicList[0]);
        PlaylistAdded();
    }

    /// <summary>
    /// 向正在播放的音乐后添加一歌曲
    /// </summary>
    /// <param name="music"></param>
    public void AddNext(Music music)
    {
        if (MusicList.Contains(music))
        {
            var c = GetCurrentIndex();
            var name = music.Name;
            Remove(music);
            Insert(GetCurrentIndex() + 1, music);
            return;
        }

        if (MusicList.Count == 0)
        {
            Add(music);
            MusicAdded?.Invoke(music);
            return;
        }

        Insert(GetCurrentIndex() + 1, music);
        MusicAdded?.Invoke(music);
        if (PlayMode is PlayModeEnum.Random) _randomMusicList.Insert(MusicList.IndexOf(CurrentMusic) + 1, music);
    }

    private void _removeMusic(Music music)
    {
        MusicList.Remove(music);
        if (PlayMode is PlayModeEnum.Random) _randomMusicList.Remove(music);

    }

    public void Add(Music music)
    {
        MusicList.Add(music);
        CurrentQueueChanged?.Invoke();
    }
    public void Insert(int index, Music music)
    {
        MusicList.Insert(index, music);
        CurrentQueueChanged?.Invoke();
    }

    public void RemoveAt(int index)
    {
        _removeMusic(MusicList[index]);
        CurrentQueueChanged?.Invoke();
    }

    public void Remove(Music music)
    {
        _removeMusic(music);
        CurrentQueueChanged?.Invoke();
    }
    public void Clear()
    {
        MusicList.Clear();
        _randomMusicList.Clear();
        CurrentQueueChanged?.Invoke();
    }

    public void Play(Music music)
    {
        if (CurrentMusic != null)
            if (music.Id == CurrentMusic.Id)
            {
                Player.Instance.Play(true);
                return;
            }

        if (!MusicList.Contains(music)) AddNext(music);


        CurrentMusic = music;
        _isUserPressed = false;
    }

    public void PlayNext(bool isUserPressed = false)
    {
        _isUserPressed = isUserPressed;
        var list = PlayMode is PlayModeEnum.Random ? _randomMusicList : MusicList;
        if (list.Count == 0) return;
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
        if (list.Count == 0) return;
        var index = list.IndexOf(CurrentMusic) - 1;
        if (index < 0) index = list.Count - 1;
        Play(list[index]);
    }

    public int GetIndex(Music music)
    {
        return MusicList.IndexOf(music);
    }

    public int GetCurrentIndex() => GetIndex(CurrentMusic);
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