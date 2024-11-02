using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.AMLL.Helpers;
using NonsPlayer.AMLL.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using Windows.Foundation;
using Windows.UI.Core;
using static System.Windows.Forms.VisualStyles.VisualStyleElement;

namespace NonsPlayer.AMLL.Views;

public sealed partial class AMLLCard : UserControl
{
    public AMLLViewModel ViewModel
    {
        get;
    }

    public AMLLCard()
    {
        ViewModel = App.GetService<AMLLViewModel>();
        InitializeComponent();
        Player.Instance.PositionChanged += OnPositionChanged;
        Player.Instance.MusicChanged += MusicChanged;
    }

    private void MusicChanged(IMusic currentmusic)
    {
        LyricHelper.Instance.LyricPosition = 0;
    }

    private void OnPositionChanged(TimeSpan time)
    {
        try
        {
            // 获取当前和下一个歌词
            if (LyricHelper.Instance.LyricPosition > ViewModel.LyricItems.Count - 1)
            {
                return;
            }
            var currentLyric = ViewModel.LyricItems[LyricHelper.Instance.LyricPosition];
            var nextLyric = LyricHelper.Instance.LyricPosition < ViewModel.LyricItems.Count - 1
                ? ViewModel.LyricItems[LyricHelper.Instance.LyricPosition + 1]
                : null;

            // 当前歌词开始和结束时间
            var startTime = currentLyric.LyricItemModel.Lyric.StartTime;
            var endTime = nextLyric?.LyricItemModel.Lyric.StartTime ?? TimeSpan.MaxValue;

            // 判断当前时间是否在当前歌词的时间范围内
            if (time >= startTime && time < endTime)
            {
                // 时间在当前歌词的时间范围内，继续显示当前歌词
                LyricHelper.Instance.LyricChanged.Invoke(currentLyric.LyricItemModel.Lyric);
            }
            else
            {
                // 时间超出了当前歌词的范围，需要更新歌词位置
                if (time >= endTime && LyricHelper.Instance.LyricPosition < ViewModel.LyricItems.Count - 1)
                {
                    LyricHelper.Instance.LyricPosition++;
                }
                else if (time < startTime && LyricHelper.Instance.LyricPosition > 0)
                {
                    LyricHelper.Instance.LyricPosition--;
                }

                // 更新歌词
                DispatcherQueue.TryEnqueue(() =>
                {
                    ScrollLyric();
                });
            }
        }catch
        {
            // ignore
        }
        
    }

    private int CalculateLyricPosition(TimeSpan time)
    {
        if (ViewModel.LyricItems.Count == 0)
        {
            return -1;
        }

        int currentPosition = LyricHelper.Instance.LyricPosition;
        int itemCount = ViewModel.LyricItems.Count;

        // 检查当前歌词的开始时间和结束时间
        if (currentPosition >= 0 && currentPosition < itemCount)
        {
            var currentLyric = ViewModel.LyricItems[currentPosition].LyricItemModel.Lyric;
            if (time >= currentLyric.StartTime && time <= currentLyric.EndTime)
            {
                return currentPosition; // 当前歌词位置正确
            }
        }

        // 检查下一个歌词的时间
        if (currentPosition + 1 < itemCount)
        {
            var nextLyric = ViewModel.LyricItems[currentPosition + 1].LyricItemModel.Lyric;
            if (time >= nextLyric.StartTime)
            {
                return currentPosition + 1; // 下一个歌词位置正确
            }
        }

        // 检查上一个歌词的时间
        if (currentPosition - 1 >= 0)
        {
            var prevLyric = ViewModel.LyricItems[currentPosition - 1].LyricItemModel.Lyric;
            if (time < prevLyric.StartTime)
            {
                return currentPosition - 1; // 上一个歌词位置正确
            }
        }

        // 遍历所有歌词项，找到当前时间对应的位置
        for (int i = 0; i < itemCount; i++)
        {
            var lyric = ViewModel.LyricItems[i].LyricItemModel.Lyric;
            if (time >= lyric.StartTime && (i == itemCount - 1 || time < ViewModel.LyricItems[i + 1].LyricItemModel.Lyric.StartTime))
            {
                return i;
            }
        }

        return -1; // 未找到合适的歌词位置
    }

    private void ScrollLyric()
    {
        try
        {
            if (LyricHelper.Instance.LyricPosition == 0)
            {
                LyricBoxContainer.ChangeView(null, 0, null, false);
                return;
            }

            var item = ViewModel.LyricItems[LyricHelper.Instance.LyricPosition];
            var k = LyricBox.ItemsSourceView.IndexOf(item);
            UIElement actualElement;
            bool isNewLoaded = false;
            if (LyricBox.TryGetElement(k) is { } ele)
            {
                actualElement = ele;
            }
            else
            {
                actualElement = LyricBox.GetOrCreateElement(k) as Border;
                isNewLoaded = true;
            }


            DispatcherQueue.TryEnqueue(() =>
            {
                actualElement.UpdateLayout();

                if (!isNewLoaded)
                {
                    var transform = actualElement?.TransformToVisual((UIElement)LyricBoxContainer.ContentTemplateRoot);
                    var position = transform?.TransformPoint(new Windows.Foundation.Point(0, 0));
                    LyricBoxContainer.ChangeView(0,
                        (position?.Y + LyricHost.Margin.Top - MainGrid.ActualHeight / 4) - 200, 1,
                        false);
                }
                else
                {
                    // actualElement.StartBringIntoView(NoAnimationBringIntoViewOptions);
                }
            });
  
        }
        catch
        {
            // igrone pls
        }
    }
}