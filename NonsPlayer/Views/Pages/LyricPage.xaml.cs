using System.Drawing;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;
using static NonsPlayer.Core.Services.ControlFactory;
using System.Text;
using Windows.Graphics.Imaging;
using ColorThiefDotNet;
using Color = Windows.UI.Color;
using NonsPlayer.Helpers;
using Microsoft.UI.Xaml;
using Windows.Storage.Streams;
using Buffer = Windows.Storage.Streams.Buffer;
using System.Runtime.InteropServices.WindowsRuntime;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.WinUI.Animations;
using Microsoft.UI.Xaml.Media;
using CommunityToolkit.WinUI.Media;

namespace NonsPlayer.Views.Pages;

[INotifyPropertyChanged]
public sealed partial class LyricPage : Page
{
    private Guid GetPictureCodecFromBuffer(Buffer buffer)
    {
        if (buffer.Length < 10) throw new ArgumentOutOfRangeException();
        var byteArray = buffer.ToArray();
        if (byteArray[0] == 0x89 && byteArray[1] == 0x50 && byteArray[2] == 0x4e &&
            byteArray[3] == 0x47)
        {
            // PNG
            return BitmapDecoder.PngDecoderId;
        }

        if (byteArray[6] == 0x4a && byteArray[7] == 0x46 && byteArray[8] == 0x49 &&
            byteArray[9] == 0x46)
        {
            // JPEG
            return BitmapDecoder.JpegDecoderId;
        }

        if (byteArray[0] == 0x52 && byteArray[1] == 0x49 && byteArray[2] == 0x46 &&
            byteArray[3] == 0x46 && byteArray[8] == 0x57)
        {
            // WEBP
            return BitmapDecoder.WebpDecoderId;
        }

        throw new ArgumentOutOfRangeException();
    }

    public LyricPage()
    {
        ViewModel = App.GetService<LyricViewModel>();
        InitializeComponent();
        Player.Instance.PositionChangedHandle += OnPositionChanged;
        Player.Instance.MusicChangedHandle += OnMusicChanged;
    }

    private ColorThief colorThief = new();
    private ManipulationStartedRoutedEventArgs? _slidingEventArgs = null;
    private TimeSpan StartingTimeSpan = TimeSpan.Zero;
    private bool _isSliding = false;
    public static Color AlbumColor;
    public LyricViewModel ViewModel { get; }

    [ObservableProperty] private SolidColorBrush foregroundAccentTextBrush =
        Application.Current.Resources["SystemControlPageTextBaseHighBrush"] as SolidColorBrush;

    [ObservableProperty] private SolidColorBrush foregroundIdleTextBrush =
        Application.Current.Resources["TextFillColorTertiaryBrush"] as SolidColorBrush;


    public async void OnMusicChanged(Music value)
    {
        // var stream = await CacheHelper.GetImageStreamFromServer(value.Album.SmallAvatarUrl);
        // var image = new Bitmap(stream.AsStream());
        // var color = colorThief.GetColor(image, ignoreWhite: false);
        // AlbumColor = Color.FromArgb(color.Color.A, color.Color.R, color.Color.G, color.Color.B);
        var imageBrush = await CacheHelper.GetImageBrushAsync(value.Album.CacheMiddleAvatarId, value.Album.AvatarUrl);
        imageBrush.Stretch = Stretch.UniformToFill;
        AcrylicCover.Fill = imageBrush;
        // double brightness = (0.299 * color.Color.A + 0.587 * color.Color.G + 0.114 * color.Color.B) / 255;
        ForegroundAccentTextBrush = new SolidColorBrush(Windows.UI.Color.FromArgb(255, 240, 240, 240));
        // if (brightness < 0.68)
        // {
        //     ForegroundAccentTextBrush =
        //         new SolidColorBrush(Windows.UI.Color.FromArgb(255, 255, 255, 255));
        //     ForegroundIdleTextBrush = new SolidColorBrush(Windows.UI.Color.FromArgb(66, 255, 255, 255));
        //     // 亮度低，使用浅色主题
        // }
        // else
        // {
        //     ForegroundAccentTextBrush = new SolidColorBrush(Windows.UI.Color.FromArgb(255, 0, 0, 0));
        //     ForegroundIdleTextBrush = new SolidColorBrush(Windows.UI.Color.FromArgb(114, 0, 0, 0));
        // }
        ViewModel.LyricPosition = 0;
        LyricBoxContainer.ChangeView(null, 0, null, false);
    }

    public void OnPositionChanged(TimeSpan position)
    {
        try
        {
            if (Player.Instance.CurrentMusic.IsEmpty) return;
            if (!_isSliding)
            {
                DispatcherQueue.TryEnqueue(() =>
                {
                    MusicStateModel.Instance.Position =
                        Player.Instance.NPMediaFoundationReader.CurrentTime.TotalSeconds;
                    CurrentTimeSlider.Value = Player.Instance.NPMediaFoundationReader.CurrentTime.TotalSeconds;
                });
            }

            // 判断是否需要滚动歌词
            if (ViewModel.LyricItems.Count == 0) return;
            var lyric = ViewModel.LyricItems[ViewModel.LyricPosition];
            // 控制ListView滚动
            if (lyric.SongLyric.PureLine.StartTime <= position)
            {
                if (ViewModel.LyricPosition < ViewModel.LyricItems.Count - 1)
                {
                    ViewModel.LyricPosition++;
                    DispatcherQueue.TryEnqueue(() =>
                    {
                        ScrollLyric();
                        UiHelper.Instance.LyricChanged?.Invoke(ViewModel.LyricPosition);
                    });
                }
            }
        }
        catch
        {
            // ignore
        }
    }

    private readonly BringIntoViewOptions NoAnimationBringIntoViewOptions =
        new BringIntoViewOptions()
        {
            VerticalAlignmentRatio = 0.5,
            AnimationDesired = false,
        };

    private void ScrollLyric()
    {
        try
        {
            if (ViewModel.LyricPosition == -1)
            {
                LyricBoxContainer.ChangeView(null, 0, null, false);
                return;
            }

            var item = ViewModel.LyricItems[ViewModel.LyricPosition];
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

            if (actualElement != null && item != null &&
                !string.IsNullOrEmpty(item.SongLyric.PureLine.CurrentLyric))
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
            }
        }
        catch
        {
            // igrone pls
        }
    }

    private void CurrentTimeSlider_OnManipulationCompleted(object sender, ManipulationCompletedRoutedEventArgs e)
    {
        _slidingEventArgs = null;
        var value = TimeSpan.FromSeconds(CurrentTimeSlider.Value);
        if (Math.Abs((value - StartingTimeSpan).TotalMilliseconds) > 250d)
        {
            PlayerService.Seek(value);
        }

        _isSliding = false;
    }

    private void CurrentTimeSlider_OnManipulationStarting(object sender, ManipulationStartingRoutedEventArgs e)
    {
        var value = TimeSpan.FromSeconds(CurrentTimeSlider.Value);
        StartingTimeSpan = value;
        PlayerService.Seek(value);
    }

    private void CurrentTimeSlider_OnManipulationStarted(object sender, ManipulationStartedRoutedEventArgs e)
    {
        _isSliding = true;
        _slidingEventArgs = e;
    }

    private void LyricPage_OnLoaded(object sender, RoutedEventArgs e)
    {
    }
}