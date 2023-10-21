using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

public partial class LyricViewModel : ObservableRecipient
{
    [ObservableProperty] private string originalLyric = string.Empty;
    [ObservableProperty] private string tranLyric = string.Empty;

    public LyricViewModel()
    {
        Player.Instance.PositionChangedHandle += LyricChanger;
        OriginalLyric = "暂未播放";
    }

    public Visibility TransVisibility => TranLyric.Equals(string.Empty) ? Visibility.Collapsed : Visibility.Visible;

    partial void OnTranLyricChanged(string value)
    {
        if (!value.Equals(string.Empty)) OnPropertyChanged(nameof(TransVisibility));
    }

    private void LyricChanger(TimeSpan time)
    {
        try
        {
            if (MusicStateModel.Instance.CurrentMusic.LyricGroup == null) return;

            var lyrics = MusicStateModel.Instance.CurrentMusic.LyricGroup.Lyrics;
            if (lyrics.Count == 1)
            {
                OriginalLyric = lyrics[0].OriginalLyric;
                return;
            }

            var left = 0;
            var right = lyrics.Count - 1;
            int middle;

            while (left <= right)
            {
                middle = (left + right) / 2;
                if (middle == lyrics.Count - 1) return;
                if (time >= lyrics[middle].Time && time < lyrics[middle + 1].Time)
                {
                    // 匹配成功，更新歌词显示
                    ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                    {
                        if (!OriginalLyric.Equals(lyrics[middle].OriginalLyric))
                        {
                            OriginalLyric = lyrics[middle].OriginalLyric;
                            TranLyric = lyrics[middle].TranLyric;
                        }
                    });

                    return;
                }

                if (time < lyrics[middle].Time)
                    right = middle - 1;
                else
                    left = middle + 1;
            }

            // 未匹配到，可以显示 "暂未播放" 或其他提示信息
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                OriginalLyric = "暂未播放";
                TranLyric = "";
            });
        }
        catch (ArgumentOutOfRangeException e)
        {
            // ignored
        }
        catch (Exception e)
        {
        }
    }
}