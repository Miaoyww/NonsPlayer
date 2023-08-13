using System.ComponentModel;
using System.Runtime.CompilerServices;
using Microsoft.UI.Xaml;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Models;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.ViewModels;

//TODO: LyricBox废弃, 功能保留
public class LyricBoxViewModel : INotifyPropertyChanged
{
    private Lyric? _currentLyric;
    private Lyric? _nextLyric;
    private string _originalLyric = string.Empty;
    private string _tranLyric = string.Empty;

    public string TranLyric
    {
        get => _tranLyric;
        set
        {
            _tranLyric = value;
            OnPropertyChanged();
            OnPropertyChanged(nameof(TransVisibility));
        }
    }

    public string OriginalLyric
    {
        get => _originalLyric;
        set
        {
            _originalLyric = value;
            OnPropertyChanged();
        }
    }

    public Visibility TransVisibility => TranLyric == "" ? Visibility.Collapsed : Visibility.Visible;

    public event PropertyChangedEventHandler? PropertyChanged;


    private void OnMusicChanged(Music currentMusic)
    {
        _currentLyric = null;
        _nextLyric = null;
    }

    private void LyricChanger(TimeSpan time)
    {
        try
        {
            if (MusicStateModel.Instance.CurrentMusic.Lyrics == null) return;

            var lyrics = MusicStateModel.Instance.CurrentMusic.Lyrics.Lrc;
            var left = 0;
            var right = lyrics.Count - 1;
            int middle;

            while (left <= right)
            {
                middle = (left + right) / 2;

                if (time >= lyrics[middle].Time && time < lyrics[middle + 1].Time)
                {
                    // 匹配成功，更新歌词显示
                    _currentLyric = lyrics[middle];
                    _nextLyric = lyrics[middle + 1];
                    OriginalLyric = lyrics[middle].OriginalLyric;
                    TranLyric = lyrics[middle].TranLyric;
                    return;
                }

                if (time < lyrics[middle].Time)
                {
                    right = middle - 1;
                }
                else
                {
                    left = middle + 1;
                }
            }

            // 未匹配到，可以显示 "暂未播放" 或其他提示信息
            OriginalLyric = "暂未播放";
            TranLyric = "";
        }
        catch (Exception e)
        {
        }
    }

    public void Init()
    {
        Player.Instance.PositionChangedHandle += LyricChanger;
        Player.Instance.MusicChangedHandle += OnMusicChanged;
        OriginalLyric = "暂未播放";
    }

    protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}