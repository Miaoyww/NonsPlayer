using System.Numerics;
using System.Runtime.InteropServices;
using Microsoft.UI;
using Microsoft.UI.Composition;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Enums;
using NonsPlayer.Core.Nons.Player;
using WinRT.Interop;
using Color = Windows.UI.Color;

namespace NonsPlayer.Helpers;

public static class AnimationHelper
{
    private static readonly Compositor _compositor = CompositionTarget.GetCompositorForCurrentThread();
    private static SpringVector3NaturalMotionAnimation _springAnimation;

    private static void UpdateSpringAnimation(float finalValue)
    {
        if (_springAnimation == null)
        {
            _springAnimation = _compositor.CreateSpringVector3Animation();
            _springAnimation.Target = "Scale";
        }

        _springAnimation.FinalValue = new Vector3(finalValue);
        _springAnimation.DampingRatio = 1f;
        _springAnimation.Period = new TimeSpan(350000);
    }

    public static void CardHide(object sender, PointerRoutedEventArgs e)
    {
        UpdateSpringAnimation(1f);

        StartAnimationIfAPIPresent(sender as UIElement, _springAnimation);
    }

    public static void CardShow(object sender, PointerRoutedEventArgs e)
    {
        UpdateSpringAnimation(1.038f);

        StartAnimationIfAPIPresent(sender as UIElement, _springAnimation);
    }

    private static void StartAnimationIfAPIPresent(UIElement sender, CompositionAnimation animation)
    {
        sender.StartAnimation(animation);
    }
}

internal class TitleBarHelper
{
    private const int WAINACTIVE = 0x00;
    private const int WAACTIVE = 0x01;
    private const int WMACTIVATE = 0x0006;

    [DllImport("user32.dll")]
    private static extern IntPtr GetActiveWindow();

    [DllImport("user32.dll", CharSet = CharSet.Auto)]
    private static extern IntPtr SendMessage(IntPtr hWnd, int msg, int wParam, IntPtr lParam);

    public static void UpdateTitleBar(ElementTheme theme)
    {
        if (App.MainWindow.ExtendsContentIntoTitleBar)
        {
            if (theme != ElementTheme.Default)
            {
                Application.Current.Resources["WindowCaptionForeground"] = theme switch
                {
                    ElementTheme.Dark => new SolidColorBrush(Colors.White),
                    ElementTheme.Light => new SolidColorBrush(Colors.Black),
                    _ => new SolidColorBrush(Colors.Transparent)
                };

                Application.Current.Resources["WindowCaptionForegroundDisabled"] = theme switch
                {
                    ElementTheme.Dark => new SolidColorBrush(Color.FromArgb(0x66, 0xFF, 0xFF, 0xFF)),
                    ElementTheme.Light => new SolidColorBrush(Color.FromArgb(0x66, 0x00, 0x00, 0x00)),
                    _ => new SolidColorBrush(Colors.Transparent)
                };

                Application.Current.Resources["WindowCaptionButtonBackgroundPointerOver"] = theme switch
                {
                    ElementTheme.Dark => new SolidColorBrush(Color.FromArgb(0x33, 0xFF, 0xFF, 0xFF)),
                    ElementTheme.Light => new SolidColorBrush(Color.FromArgb(0x33, 0x00, 0x00, 0x00)),
                    _ => new SolidColorBrush(Colors.Transparent)
                };

                Application.Current.Resources["WindowCaptionButtonBackgroundPressed"] = theme switch
                {
                    ElementTheme.Dark => new SolidColorBrush(Color.FromArgb(0x66, 0xFF, 0xFF, 0xFF)),
                    ElementTheme.Light => new SolidColorBrush(Color.FromArgb(0x66, 0x00, 0x00, 0x00)),
                    _ => new SolidColorBrush(Colors.Transparent)
                };

                Application.Current.Resources["WindowCaptionButtonStrokePointerOver"] = theme switch
                {
                    ElementTheme.Dark => new SolidColorBrush(Colors.White),
                    ElementTheme.Light => new SolidColorBrush(Colors.Black),
                    _ => new SolidColorBrush(Colors.Transparent)
                };

                Application.Current.Resources["WindowCaptionButtonStrokePressed"] = theme switch
                {
                    ElementTheme.Dark => new SolidColorBrush(Colors.White),
                    ElementTheme.Light => new SolidColorBrush(Colors.Black),
                    _ => new SolidColorBrush(Colors.Transparent)
                };
            }

            Application.Current.Resources["WindowCaptionBackground"] = new SolidColorBrush(Colors.Transparent);
            Application.Current.Resources["WindowCaptionBackgroundDisabled"] = new SolidColorBrush(Colors.Transparent);

            var hwnd = WindowNative.GetWindowHandle(App.MainWindow);
            if (hwnd == GetActiveWindow())
            {
                SendMessage(hwnd, WMACTIVATE, WAINACTIVE, IntPtr.Zero);
                SendMessage(hwnd, WMACTIVATE, WAACTIVE, IntPtr.Zero);
            }
            else
            {
                SendMessage(hwnd, WMACTIVATE, WAACTIVE, IntPtr.Zero);
                SendMessage(hwnd, WMACTIVATE, WAINACTIVE, IntPtr.Zero);
            }
        }
    }
}

public class PlayerIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if ((bool)value)
            // 正在播放
            return "\uf8ae";
        // 未播放
        return "\uf5b0";
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return value.ToString() == "\uf8ae";
    }
}

public class VolumeIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        return (double)value switch
        {
            var n when n <= 10 => "\ue992",
            var n when n is <= 50 and > 10 => "\ue993",
            var n when n is <= 100 and > 50 => "\ue994",
            _ => "\ue992"
        };
    }

    public object? ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return null;
    }
}

public class LikeIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if ((bool)value) return "\uEB52";

        return "\uEB51";
    }

    public object? ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return null;
    }
}

public class PlayModeIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var mode = (PlayQueue.PlayModeEnum)value;
        switch (mode)
        {
            case PlayQueue.PlayModeEnum.Sequential:
                return "\uf5e7";
            case PlayQueue.PlayModeEnum.Recommend:
                return "\ue704";
            case PlayQueue.PlayModeEnum.SingleLoop:
                return "\ue8ed";
            case PlayQueue.PlayModeEnum.ListLoop:
                return "\ue8ee";
            default:
                return "\ue8ee";
        }
    }

    public object? ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return null;
    }
}

public class ShuffleIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var isShuffle = (bool)value;
        if (isShuffle) return Visibility.Visible;

        return Visibility.Collapsed;
    }

    public object? ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return null;
    }
}

public class SearchDataTypeToString : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var item = value as SearchDataType?;
        switch (item)
        {
            case SearchDataType.Music:
                return "单曲";
            case SearchDataType.Artist:
                return "艺术家";
            case SearchDataType.Playlist:
                return "歌单";
            case SearchDataType.Album:
                return "专辑";
            default:
                return "未知";
        }
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return "未知";
    }
}