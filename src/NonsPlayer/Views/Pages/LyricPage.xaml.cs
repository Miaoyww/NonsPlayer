using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;
using ColorThiefDotNet;
using Color = Windows.UI.Color;
using NonsPlayer.Helpers;
using Microsoft.UI.Xaml;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Contracts.Models.Music;

namespace NonsPlayer.Views.Pages;

[INotifyPropertyChanged]
public sealed partial class LyricPage : Page
{
    public LyricPage()
    {
        ViewModel = App.GetService<LyricViewModel>();
        InitializeComponent();
        Player.Instance.PositionChanged += OnPositionChanged;
        Player.Instance.MusicChanged += OnMusicChanged;
        if (MusicStateModel.Instance.CurrentMusic != null)
        {
            OnMusicChanged(MusicStateModel.Instance.CurrentMusic);
        }
    }

    private ColorThief colorThief = new();
    private ManipulationStartedRoutedEventArgs? _slidingEventArgs = null;
    private TimeSpan StartingTimeSpan = TimeSpan.Zero;
    private bool _isSliding = false;
    public static Color AlbumColor;
    public LyricViewModel ViewModel { get; }

    [ObservableProperty] private SolidColorBrush foregroundAccentTextBrush =
        Application.Current.Resources["SystemControlPageTextBaseHighBrush"] as SolidColorBrush;

    [ObservableProperty] private SolidColorBrush textForeground =
        Application.Current.Resources["TextFillColorTertiaryBrush"] as SolidColorBrush;


    public async void OnMusicChanged(IMusic value)
    {
        // var stream = await CacheHelper.GetImageStreamFromServer(value.Album.SmallAvatarUrl);
        // var image = new Bitmap(stream.AsStream());
        // var color = colorThief.GetColor(image, ignoreWhite: false);
        // AlbumColor = Color.FromArgb(color.Color.A, color.Color.R, color.Color.G, color.Color.B);
        var imageBrush = await CacheHelper.GetImageBrushAsync(value.Album.CacheMiddleAvatarId, value.Album.AvatarUrl);
        imageBrush.Stretch = Stretch.UniformToFill;
        AcrylicCover.Fill = imageBrush;
        // double brightness = (0.299 * color.Color.A + 0.587 * color.Color.G + 0.114 * color.Color.B) / 255;
        TextForeground = new SolidColorBrush(Windows.UI.Color.FromArgb(255, 240, 240, 240));
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
        // ViewModel.LyricPosition = 0;
        // LyricBoxContainer.ChangeView(null, 0, null, false);
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
                    CurrentTimeSlider.Value = Player.Instance.Position.TotalSeconds;
                });
            }
        }
        catch
        {
            // ignore
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

    [RelayCommand]
    public void UnExpand()
    {
        ServiceHelper.NavigationService.GoBack();
       
    }
}