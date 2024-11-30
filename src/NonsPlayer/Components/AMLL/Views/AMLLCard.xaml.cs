using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.AMLL.Helpers;
using NonsPlayer.Components.AMLL.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using Windows.Foundation;
using Windows.UI.Core;
using static System.Windows.Forms.VisualStyles.VisualStyleElement;

namespace NonsPlayer.Components.AMLL.Views;

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
            if (LyricBox.ItemsSourceView != null)
            {
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

  
        }
        catch
        {
            // igrone pls
        }
    }
}