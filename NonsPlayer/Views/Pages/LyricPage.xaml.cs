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
    }

    public void OnPositionChanged(TimeSpan position)
    {
        if (Player.Instance.CurrentMusic.IsEmpty) return;
        if (!_isSliding)
        {
            DispatcherQueue.TryEnqueue(() =>
            {
                MusicStateModel.Instance.Position = Player.Instance.NPMediaFoundationReader.CurrentTime.TotalSeconds;
                CurrentTimeSlider.Value = Player.Instance.NPMediaFoundationReader.CurrentTime.TotalSeconds;
            });
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