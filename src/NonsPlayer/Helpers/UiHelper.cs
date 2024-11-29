using CommunityToolkit.Mvvm.ComponentModel;
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
                App.MainWindow.AppWindow.TitleBar.ButtonForegroundColor = theme switch
                {
                    ElementTheme.Dark => Colors.White,
                    ElementTheme.Light => Colors.Black,
                    _ => Colors.Transparent
                };

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

public partial class UiHelper : ObservableObject
{
    public static UiHelper Instance { get; } = new();

    // 构建歌词修改时间
    public delegate void LyricChangedHandler(int index);

    public LyricChangedHandler LyricChanged;
    
    [ObservableProperty] private Visibility lyricShow = Visibility.Collapsed;
    [ObservableProperty] private Visibility playBarShow = Visibility.Visible;

    partial void OnLyricShowChanged(Visibility value)
    {
        switch (value)
        {
            case Visibility.Collapsed:
                PlayBarShow = Visibility.Visible;
                break;
            case Visibility.Visible:
                PlayBarShow = Visibility.Collapsed;
                break;
        }
    }
}