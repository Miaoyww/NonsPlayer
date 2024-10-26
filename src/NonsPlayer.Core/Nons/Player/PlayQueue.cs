﻿using NAudio.Wave;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;

namespace NonsPlayer.Core.Nons.Player;

public class PlayQueue
{
    #region 事件注册

    public delegate void CurrentMusicChangedEventHandler(IMusic value);

    public delegate void MusicAddedEventHandler(IMusic value);

    public delegate void PlaylistAddedEventHandler();

    public delegate void PlayModeChangedEventHandler(PlayModeEnum mode);

    public delegate void PlayQueueChangedHandler();

    public delegate void ShuffleChangedEventHandler(bool isShuffle);

    public delegate void RadioWaitEventHandler();

    public event CurrentMusicChangedEventHandler CurrentMusicChanged;

    public event PlayQueueChangedHandler CurrentQueueChanged;

    public event MusicAddedEventHandler MusicAdded;

    public event PlaylistAddedEventHandler PlaylistAdded;

    public event PlayModeChangedEventHandler PlayModeChanged;

    public event ShuffleChangedEventHandler ShuffleChanged;

    public event RadioWaitEventHandler RadioWaiting;

    #endregion

    #region 事件回应

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
            if (Player.Instance.IsMixed)
            {
                Player.Instance.LoadNextTrack();
                return;
            }

            if (CurrentMusic is null) return;
            if (_isUserPressed) return;
            if (CurrentMusic.Duration.TotalSeconds - Player.Instance.Position.TotalSeconds > 2) return;
            if (Player.Instance.IsMixed)
            {
                return;
            }

            switch (PlayMode)
            {
                case PlayModeEnum.ListLoop:
                    if (Player.Instance.CurrentMusic != Player.Instance.PreviousMusic &&
                        PlayMode != PlayModeEnum.SingleLoop)
                    {
                        if (_isUserPressed) return;


                        PlayNext();
                    }

                    break;
                case PlayModeEnum.SingleLoop:
                    Play(CurrentMusic);
                    break;
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
            if (Player.Instance.IsMixed)
            {
                Player.Instance.LoadNextTrack();
            }

            await Player.Instance.NewPlay(value);
        }
        catch (Exception e)
        {
            ExceptionService.Instance.Throw(e);
            PlayNext();
        }
    }

    #endregion

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

    public bool IsRadioMode;
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

    public void AddMusicList(IMusic[] musicList, bool play = false)
    {
        MusicList.AddRange(musicList);
        if (play) Play(musicList[0]);
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

    private void RemoveMusic(IMusic music)
    {
        // 删除歌曲
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
        RemoveMusic(MusicList[index]);
        CurrentQueueChanged?.Invoke();
    }

    public void Remove(IMusic music)
    {
        RemoveMusic(music);
        CurrentQueueChanged?.Invoke();
    }

    public void Clear()
    {
        MusicList.Clear();
        _randomMusicList.Clear();
        CurrentQueueChanged?.Invoke();
    }

    public async void Play(IMusic music)
    {
        if (GetCurrentIndex() == MusicList.Count - 1)
        {
            if (IsRadioMode)
            {
                RadioWaiting?.Invoke();
                return;
            }
        }

        if (CurrentMusic != null)
        {
            if (music.Id == CurrentMusic.Id)
            {
                await Player.Instance.Play(true);
                return;
            }
        }


        if (!MusicList.Contains(music)) AddNext(music);


        CurrentMusic = music;
        _isUserPressed = false;
    }

    public void PlayNext(bool isUserPressed = false)
    {
        _isUserPressed = isUserPressed;
        PlayModeEnum playMode = PlayMode;

        if (isUserPressed)
        {
            playMode = PlayModeEnum.ListLoop;
        }
        
        var list = IsShuffle ? _randomMusicList : MusicList;
        if (list.Count == 0) return;
        var index = list.IndexOf(CurrentMusic) + 1;

        switch (playMode)
        {
            case PlayModeEnum.SingleLoop:
                Play(CurrentMusic);
                break;
            case PlayModeEnum.Sequential:
                if (index >= list.Count) return;
                Play(list[index]);
                break;
            case PlayModeEnum.ListLoop:
                if (list.Count == 0) return;
                index = index >= list.Count ? 0 : index;
                Play(list[index]);
                break;
            case PlayModeEnum.Recommend:
                break;
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