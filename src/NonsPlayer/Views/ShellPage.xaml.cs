using Windows.System;
using CommunityToolkit.Mvvm.Input;
using Gma.System.MouseKeyHook;
using Microsoft.UI;
using Microsoft.UI.Input;
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
using Vanara.PInvoke;
using Windows.Graphics;
using Windows.UI.WindowManagement;
using Windows.Win32;
using Windows.Win32.Foundation;
using AppWindow = Microsoft.UI.Windowing.AppWindow;
using AppWindowTitleBar = Microsoft.UI.Windowing.AppWindowTitleBar;

namespace NonsPlayer.Views;

public sealed partial class ShellPage : Page
{
    public delegate void ShellViewDialogShowEventHandler();

    public UiHelper UiHelper = UiHelper.Instance;
    private AppWindow AppWindow => Window.AppWindow;

    private Window Window;

    public ShellPage()
    {
        ViewModel = App.GetService<ShellViewModel>();
        InitializeComponent();
        PlayQueueBarViewModel = App.GetService<PlayQueueBarViewModel>();
        ViewModel.NavigationService.Frame = NavigationFrame;
        App.WindowHandle = (IntPtr)App.MainWindow.AppWindow.Id.Value;
        WindowUtility.CurrentWindowId = Win32Interop.GetWindowIdFromWindow(App.WindowHandle);

        App.MainWindow.ExtendsContentIntoTitleBar = true;
        App.MainWindow.AppWindow.TitleBar.IconShowOptions = IconShowOptions.ShowIconAndSystemMenu;
        App.MainWindow.AppWindow.TitleBar.PreferredHeightOption = TitleBarHeightOption.Tall;
        App.MainWindow.Activated += MainWindow_Activated;
        App.MainWindow.SetTitleBar(TitleIcon);
        // SetDragRectangles(new RectInt32(0, 0, 100000, (int)(48 * WindowUtility.UiScale)));
        AppTitleBarText.Text = "AppDisplayName".GetLocalized();
        PlayerBar.OnPlayQueueBarOpenHandler += OnOpenPlayQueueButton_Click;
        ExceptionService.Instance.ExceptionThrew += OnExceptionThrew;
        DialogHelper.Instance.DialogShowing += OnDialogShowing;
    }


    public void SetDragRectangles(params RectInt32[] value)
    {
        if (AppWindowTitleBar.IsCustomizationSupported() &&
            App.MainWindow.AppWindow.TitleBar.ExtendsContentIntoTitleBar == true)
        {
            App.MainWindow.AppWindow.TitleBar.SetDragRectangles(value);
        }
    }

    public ShellViewModel ViewModel { get; }

    public PlayQueueBarViewModel PlayQueueBarViewModel { get; }

    private void OnDialogShowing(string content)
    {
        ExceptionTip.DispatcherQueue.TryEnqueue(() =>
        {
            ExceptionTip.Title = "Notification".GetLocalized();
            ExceptionTip.Content = content;
            ExceptionTip.IsOpen = true;
        });
    }

    private void OnExceptionThrew(string content)
    {
        ExceptionTip.DispatcherQueue.TryEnqueue(() =>
        {
            ExceptionTip.Title = "Error".GetLocalized();
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
        // var resource = args.WindowActivationState == WindowActivationState.Deactivated
        //     ? "WindowCaptionForegroundDisabled"
        //     : "WindowCaptionForeground";

        // AppTitleBarText.Foreground = (SolidColorBrush)App.Current.Resources[resource];
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

    private void ShellPage_OnSizeChanged(object sender, SizeChangedEventArgs e)
    {
        App.MainWindow.RaiseSetTitleBarDragRegion(SetTitleBarDragRegion);
    }

    private int SetTitleBarDragRegion(InputNonClientPointerSource source, SizeInt32 size, double scaleFactor,
        Func<UIElement, RectInt32?, RectInt32> getScaledRect)
    {
        // source.SetRegionRects(NonClientRegionKind.Passthrough, [getScaledRect(SearchBar, null)]);
        // source.SetRegionRects(NonClientRegionKind.Passthrough, [getScaledRect(SettingsBar, null)]);
        // source.SetRegionRects(NonClientRegionKind.Passthrough, [getScaledRect(NavigationBar, null)]);
        return 48;
    }
}