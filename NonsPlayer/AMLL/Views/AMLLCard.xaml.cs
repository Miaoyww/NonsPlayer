using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.AMLL.Helpers;
using NonsPlayer.AMLL.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;

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
        // 判断是否需要滚动歌词
        if (ViewModel.LyricItems.Count == 0) return;

        var lyric = ViewModel.LyricItems[LyricHelper.Instance.LyricPosition];
        // 控制ListView滚动
        if (lyric.LyricItemModel.Lyric.StartTime <= time)
        {
            if (LyricHelper.Instance.LyricPosition < ViewModel.LyricItems.Count - 1)
            {
                LyricHelper.Instance.LyricPosition++;
                LyricHelper.Instance.LyricChanged.Invoke(lyric.LyricItemModel.Lyric);
                DispatcherQueue.TryEnqueue(() =>
                {
                    ScrollLyric();
                });
            }
        }
    }

    private void ScrollLyric()
    {
        try
        {
            try
            {
                if (LyricHelper.Instance.LyricPosition == -1)
                {
                    LyricBoxContainer.ChangeView(null, 0, null, false);
                    return;
                }

                var item = LyricBox.Items[LyricHelper.Instance.LyricPosition];
                DispatcherQueue.TryEnqueue(() =>
                {
                    LyricBox.ScrollIntoView(item);
                });
            }
            catch (Exception ex)
            {
                // Log the error if needed
            }

            // var item = ViewModel.LyricItems[ViewModel.LyricPosition];
            // var k = LyricBox.ItemsSourceView.IndexOf(item);
            // UIElement actualElement;
            // bool isNewLoaded = false;
            // if (LyricBox.TryGetElement(k) is { } ele)
            // {
            //     actualElement = ele;
            // }
            // else
            // {
            //     actualElement = LyricBox.GetOrCreateElement(k) as Border;
            //     isNewLoaded = true;
            // }
            //
            // if (actualElement != null && item != null &&
            //     !string.IsNullOrEmpty(item.LyricItemModel.Lyric.Pure))
            // {
            //     actualElement.UpdateLayout();
            //     actualElement.StartBringIntoView();
            //
            //     if (!isNewLoaded)
            //     {
            //         var transform = actualElement?.TransformToVisual((UIElement)LyricBoxContainer.ContentTemplateRoot);
            //         var position = transform?.TransformPoint(new Windows.Foundation.Point(0, 0));
            //         LyricBoxContainer.ChangeView(0,
            //             (position?.Y + LyricHost.Margin.Top - MainGrid.ActualHeight / 4) - 200, 1,
            //             false);
            //     }
            //     else
            //     {
            //         // actualElement.StartBringIntoView(NoAnimationBringIntoViewOptions);
            //     }
            // }
        }
        catch
        {
            // igrone pls
        }
    }
}