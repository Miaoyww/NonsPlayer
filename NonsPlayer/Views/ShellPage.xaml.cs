using Windows.System;
using CommunityToolkit.Mvvm.Input;
using Gma.System.MouseKeyHook;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using Microsoft.UI.Windowing;
using Windows.Graphics;
using Windows.Win32;
using Windows.Win32.Foundation;
using AppWindow = Microsoft.UI.Windowing.AppWindow;
using AppWindowChangedEventArgs = Microsoft.UI.Windowing.AppWindowChangedEventArgs;
using AppWindowTitleBar = Microsoft.UI.Windowing.AppWindowTitleBar;
using Microsoft.UI.Input;
using Windows.Foundation;

namespace NonsPlayer.Views;

public sealed partial class ShellPage : Page
{
    public delegate void ShellViewDialogShowEventHandler();

    public IntPtr WindowHandle { get; private init; }

    public double UIScale => PInvoke.GetDpiForWindow((HWND)WindowHandle) / 96d;

    public ShellPage(ShellViewModel viewModel)
    {
        ViewModel = viewModel;
        InitializeComponent();
        PlayQueueBarViewModel = App.GetService<PlayQueueBarViewModel>();
        ViewModel.NavigationService.Frame = NavigationFrame;
        WindowHandle = (IntPtr)App.MainWindow.AppWindow.Id.Value;
        AppTitleBar.SizeChanged += AppTitleBarOnSizeChanged;
        App.MainWindow.AppWindow.Changed += AppWindowOnChanged;
        App.MainWindow.ExtendsContentIntoTitleBar = true;
        App.MainWindow.Activated += MainWindow_Activated;

        AppTitleBarText.Text = "AppDisplayName".GetLocalized();
        App.MainWindow.AppWindow.TitleBar.IconShowOptions = IconShowOptions.HideIconAndSystemMenu;
        App.MainWindow.AppWindow.TitleBar.PreferredHeightOption = TitleBarHeightOption.Tall;
        PlayerBar.OnPlayQueueBarOpenHandler += OnOpenPlayQueueButton_Click;
        ExceptionService.Instance.ExceptionThrew += OnExceptionThrew;

        SetRegionsForCustomTitleBar();
    }

    private void AppTitleBarOnSizeChanged(object sender, SizeChangedEventArgs e)
    {
        SetRegionsForCustomTitleBar();
    }

    private void SetRegionsForCustomTitleBar()
    {
        // 获取缩放比例
        double scaleAdjustment = this.XamlRoot.RasterizationScale;

        // 调整左右填充列的宽度，以适应系统插图
        SettingsColumn.Width = new GridLength(App.MainWindow.AppWindow.TitleBar.RightInset / scaleAdjustment);
        TitleBarColumn.Width = new GridLength(App.MainWindow.AppWindow.TitleBar.LeftInset / scaleAdjustment);

        // 获取搜索框的交互区域
        GeneralTransform transform = Bars.TransformToVisual(null);
        Rect bounds = transform.TransformBounds(new Rect(0, 0,
            Bars.ActualWidth,
            Bars.ActualHeight));
        Windows.Graphics.RectInt32 SearchBoxRect = GetRect(bounds, scaleAdjustment);

        // 获取账户图片的交互区域
        transform = AppTitleBar.TransformToVisual(null);
        bounds = transform.TransformBounds(new Rect(0, 0,
            AppTitleBar.ActualWidth,
            AppTitleBar.ActualHeight));
        RectInt32 PersonPicRect = GetRect(bounds, scaleAdjustment);

        // 定义交互区域数组
        var rectArray = new [] { SearchBoxRect, PersonPicRect };

        // 设置非客户端区域的交互区域
        InputNonClientPointerSource nonClientInputSrc =
            InputNonClientPointerSource.GetForWindowId(App.MainWindow.AppWindow.Id);
        nonClientInputSrc.SetRegionRects(NonClientRegionKind.Passthrough, rectArray);
    }

    private Windows.Graphics.RectInt32 GetRect(Rect bounds, double scale)
    {
        return new Windows.Graphics.RectInt32(
            _X: (int)Math.Round(bounds.X * scale),
            _Y: (int)Math.Round(bounds.Y * scale),
            _Width: (int)Math.Round(bounds.Width * scale),
            _Height: (int)Math.Round(bounds.Height * scale)
        );
    }
    private void AppWindowOnChanged(AppWindow sender, AppWindowChangedEventArgs args)
    {
        SetRegionsForCustomTitleBar();
    }


    public ShellViewModel ViewModel { get; }

    public PlayQueueBarViewModel PlayQueueBarViewModel { get; }

    private void OnExceptionThrew(string content)
    {
        ExceptionTip.DispatcherQueue.TryEnqueue(() =>
        {
            ExceptionTip.Title = "出错啦！";
            ExceptionTip.Content = content;
            ExceptionTip.IsOpen = true;
        });
    }

    [RelayCommand]
    public void ExceptionAction()
    {
        ExceptionTip.IsOpen = false;
    }

    [RelayCommand]
    public void NavigateMe()
    {
        ViewModel.NavigationService.NavigateTo(typeof(PersonalCenterViewModel).FullName);
    }

    public event ShellViewDialogShowEventHandler OnDialogShowCall;

    public void OnOpenPlayQueueButton_Click(object? sender, EventArgs e)
    {
        PlayQueueBar.IsPaneOpen = !PlayQueueBar.IsPaneOpen;
    }


    private async void OnLoaded(object sender, RoutedEventArgs e)
    {
        TitleBarHelper.UpdateTitleBar(RequestedTheme);

        KeyboardAccelerators.Add(BuildKeyboardAccelerator(VirtualKey.Left, VirtualKeyModifiers.Menu));
        KeyboardAccelerators.Add(BuildKeyboardAccelerator(VirtualKey.GoBack));
        PlayQueue.Instance.CurrentMusicChanged += OnCurrentMusicChanged;
        ExceptionService.Instance.ExceptionThrew += OnExceptionThrew;
    }

    private void OnCurrentMusicChanged(IMusic value)
    {
        if (value.IsEmpty) return;

        App.MainWindow.Title = "AppDisplayName".GetLocalized() + " - " + value.Name;
    }

    private void MainWindow_Activated(object sender, WindowActivatedEventArgs args)
    {
        
    }

    private void OnUnloaded(object sender, RoutedEventArgs e)
    {
        Hook.GlobalEvents().Dispose();
        App.GetService<ControlService>().Stop();
        Player.Instance.Dispose();
        ConfigManager.Instance.Save();
    }

    private static KeyboardAccelerator BuildKeyboardAccelerator(VirtualKey key, VirtualKeyModifiers? modifiers = null)
    {
        var keyboardAccelerator = new KeyboardAccelerator { Key = key };

        if (modifiers.HasValue) keyboardAccelerator.Modifiers = modifiers.Value;

        keyboardAccelerator.Invoked += OnKeyboardAcceleratorInvoked;

        return keyboardAccelerator;
    }

    private static void OnKeyboardAcceleratorInvoked(KeyboardAccelerator sender,
        KeyboardAcceleratorInvokedEventArgs args)
    {
        var navigationService = App.GetService<INavigationService>();

        var result = navigationService.GoBack();

        args.Handled = result;
    }
}