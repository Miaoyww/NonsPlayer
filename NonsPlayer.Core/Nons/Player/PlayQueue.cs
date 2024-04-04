using NAudio.Wave;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;

namespace NonsPlayer.Core.Nons.Player;

public class PlayQueue
{
    public delegate void CurrentMusicChangedEventHandler(IMusic value);

    public delegate void MusicAddedEventHandler(IMusic value);

    public delegate void PlaylistAddedEventHandler();

    public delegate void PlayModeChangedEventHandler(PlayModeEnum mode);

    public delegate void PlayQueueChangedHandler();

    public delegate void ShuffleChangedEventHandler(bool isShuffle);

    public enum PlayModeEnum
    {
        Sequential, //顺序播放
        SingleLoop, //单曲循环
        ListLoop, //列表循环
        Recommend
    }

    private IMusic _currentMusic;

    private bool _isUserPressed;
    private List<IMusic> _randomMusicList = new();

    public PlayQueue()
    {
        MusicList = new List<IMusic>();
        PlayMode = PlayModeEnum.ListLoop;
        Player.Instance.OutputDevice.PlaybackStopped += OnMusicStopped;
        CurrentMusicChanged += OnCurrentMusicChanged;
        MusicAdded += OnMusicAdded;
        PlaylistAdded += OnPlaylistAdded;
        PlayModeChanged += OnPlayModeChanged;
    }

    public bool IsShuffle { get; private set; }


    public static PlayQueue Instance { get; } = new();

    public List<IMusic> MusicList { get; set; }
    public int Count => MusicList.Count;

    public IMusic CurrentMusic
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

    public event PlayModeChangedEventHandler PlayModeChanged;
    public event ShuffleChangedEventHandler ShuffleChanged;

    private void OnPlaylistAdded()
    {
        CurrentQueueChanged?.Invoke();
    }

    private void OnMusicAdded(IMusic value)
    {
        CurrentQueueChanged?.Invoke();
    }

    private void OnPlayModeChanged(PlayModeEnum mode)
    {
        if (mode == PlayModeEnum.Recommend && IsShuffle)
        {
            IsShuffle = false;
            ShuffleChanged?.Invoke(false);
        }
    }

    public void OnMusicStopped(object sender, StoppedEventArgs e)
    {
        try
        {
            if (PlayMode == PlayModeEnum.ListLoop)
                if (Player.Instance.CurrentMusic != Player.Instance.PreviousMusic &&
                    PlayMode != PlayModeEnum.SingleLoop)
                {
                    if (_isUserPressed) return;
                    if (CurrentMusic.Duration.TotalSeconds - Player.Instance.Position.TotalSeconds > 1) return;
                    PlayNext();
                }
        }
        catch (Exception exception)
        {
            ExceptionService.Instance.Throw(exception);
        }
    }

    // 当CurrentMusic改变时，将触发PlayerService的NewPlay方法
    public async void OnCurrentMusicChanged(IMusic value)
    {
        try
        {
            await Player.Instance.NewPlay(value);
        }
        catch (Exception e)
        {
            ExceptionService.Instance.Throw(e);
            PlayNext();
        }
    }

    public void SwitchShuffle()
    {
        if (PlayMode == PlayModeEnum.Recommend && !IsShuffle)
        {
            IsShuffle = false;
            ShuffleChanged?.Invoke(false);
            return;
        }

        IsShuffle = !IsShuffle;
        ShuffleChanged?.Invoke(IsShuffle);
    }

    public void SwitchPlayMode()
    {
        PlayMode = PlayMode switch
        {
            PlayModeEnum.ListLoop => PlayModeEnum.SingleLoop,
            PlayModeEnum.SingleLoop => PlayModeEnum.Sequential,
            PlayModeEnum.Sequential => PlayModeEnum.Recommend,
            PlayModeEnum.Recommend => PlayModeEnum.ListLoop,
            _ => PlayModeEnum.ListLoop
        };
        // 如果模式是随机播放，调用GetRandomList方法
        if (IsShuffle) _randomMusicList = GetRandomList(CurrentMusic);
        PlayModeChanged?.Invoke(PlayMode);
    }

    public void AddMusicList(IMusic[] musicList)
    {
        Clear();
        MusicList.AddRange(musicList);
        Play(musicList[0]);
        PlaylistAdded();
    }

    /// <summary>
    ///     向正在播放的音乐后添加一歌曲
    /// </summary>
    /// <param name="music"></param>
    public void AddNext(IMusic music)
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
        if (IsShuffle) _randomMusicList.Insert(MusicList.IndexOf(CurrentMusic) + 1, music);
    }

    public void AddNext(IMusic[] musics)
    {
        var content = musics.Reverse();
        foreach (var item in content) AddNext(item);
    }

    private void _removeMusic(IMusic music)
    {
        MusicList.Remove(music);
        if (IsShuffle) _randomMusicList.Remove(music);
    }

    public void Add(IMusic music)
    {
        MusicList.Add(music);
        CurrentQueueChanged?.Invoke();
    }

    public void Insert(int index, IMusic music)
    {
        MusicList.Insert(index, music);
        CurrentQueueChanged?.Invoke();
    }

    public void RemoveAt(int index)
    {
        _removeMusic(MusicList[index]);
        CurrentQueueChanged?.Invoke();
    }

    public void Remove(IMusic music)
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

    public void Play(IMusic music)
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
        var list = IsShuffle ? _randomMusicList : MusicList;
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

        //TODO: 长期任务: 完成Recommend模式
        if (PlayMode is PlayModeEnum.Recommend)
        {
            Play(CurrentMusic);
        }
    }

    public void PlayPrevious(bool isUserPressed = false)
    {
        _isUserPressed = isUserPressed;
        var list = IsShuffle ? _randomMusicList : MusicList;
        if (list.Count == 0) return;
        var index = list.IndexOf(CurrentMusic) - 1;
        if (index < 0) index = list.Count - 1;
        Play(list[index]);
    }

    public int GetIndex(IMusic music)
    {
        return MusicList.IndexOf(music);
    }

    public int GetCurrentIndex()
    {
        return GetIndex(CurrentMusic);
    }

    /// <summary>
    ///     获取随机播放列表
    /// </summary>
    /// <param name="music">当前播放的歌曲</param>
    private List<IMusic> GetRandomList(IMusic music)
    {
        return new List<IMusic>();
        // var random = new Random();
        // var list = MusicList.ToList();
        // list.RemoveAt(0);
        // list = list.OrderBy(x => random.Next()).ToList();
        // list.Insert(0, music);
        // return list;
    }
}