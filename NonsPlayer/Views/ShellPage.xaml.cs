using Windows.System;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Exceptions;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Account;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views;

public sealed partial class ShellPage : Page
{
    public delegate void ShellViewDialogShowEventHandler();

    public ShellPage(ShellViewModel viewModel)
    {
        ViewModel = viewModel;
        InitializeComponent();
        PlayQueueBarViewModel = App.GetService<PlayQueueBarViewModel>();
        ViewModel.NavigationService.Frame = NavigationFrame;

        // TODO: Set the title bar icon by updating /Assets/WindowIcon.ico.
        // A custom title bar is required for full window theme and Mica support.
        // https://docs.microsoft.com/windows/apps/develop/title-bar?tabs=winui3#full-customization
        App.MainWindow.ExtendsContentIntoTitleBar = true;
        App.MainWindow.SetTitleBar(AppTitleBar);
        App.MainWindow.Activated += MainWindow_Activated;
        AppTitleBarText.Text = "AppDisplayName".GetLocalized();
        PlayerBar.OnPlayQueueBarOpenHandler += OnOpenPlayQueueButton_Click;
        ExceptionService.Instance.ExceptionThrew += OnExceptionThrew;
    }

    public ShellViewModel ViewModel { get; }

    public PlayQueueBarViewModel PlayQueueBarViewModel { get; }

    private void OnExceptionThrew(string content)
    {
        ExceptionTtp.DispatcherQueue.TryEnqueue(() =>
        {
            ExceptionTtp.Title = "出错啦！";
            ExceptionTtp.Content = content;
            ExceptionTtp.IsOpen = true;
        });
    }

    [RelayCommand]
    public void ExceptionAction()
    {
        ExceptionTtp.IsOpen = false;
    }

    [RelayCommand]
    public void NavigateMe()
    {
        ViewModel.NavigationService.NavigateTo(typeof(PersonalCenterViewModel).FullName);
    }

    [RelayCommand]
    public async Task SignOut()
    {
        if (!Account.Instance.IsLoggedIn)
        {
            ExceptionTtp.Title = "出错啦！";
            ExceptionTtp.Content =  "当前未登录哦.";
            ExceptionTtp.IsOpen = true;
            return;
        }
        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot,
            Style = Application.Current.Resources["DefaultContentDialogStyle"] as Style,
            Title = "确认",
            PrimaryButtonText = "确定",
            CloseButtonText = "点错了",
            DefaultButton = ContentDialogButton.Primary,
            Content = "确定要退出当前账号吗?"
        };
        var result = await dialog.ShowAsync();
        if (result == ContentDialogResult.Primary)
        {
            await Account.Instance.LogOut();
        }
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

        ShellMenuBarSettingsButton.AddHandler(PointerPressedEvent,
            new PointerEventHandler(ShellMenuBarSettingsButton_PointerPressed), true);
        ShellMenuBarSettingsButton.AddHandler(PointerReleasedEvent,
            new PointerEventHandler(ShellMenuBarSettingsButton_PointerReleased), true);
        PlayQueue.Instance.CurrentMusicChanged += OnCurrentMusicChanged;
        ExceptionService.Instance.ExceptionThrew += OnExceptionThrew;
        AccountService.Instance.UpdateInfo();

        try
        {
            await Account.Instance.LoginByReg();
        }
        catch (LoginFailureException error)
        {
            var dialog = new ContentDialog
            {
                XamlRoot = XamlRoot,
                Style = Application.Current.Resources["DefaultContentDialogStyle"] as Style,
                Title = "错误",
                PrimaryButtonText = "知道了",
                CloseButtonText = "取消",
                DefaultButton = ContentDialogButton.Primary,
                Content = $"登录失败~ {error.Message}"
            };
            await dialog.ShowAsync();
            await Account.Instance.LogOut();
        }
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
        ShellMenuBarSettingsButton.RemoveHandler(PointerPressedEvent,
            (PointerEventHandler)ShellMenuBarSettingsButton_PointerPressed);
        ShellMenuBarSettingsButton.RemoveHandler(PointerReleasedEvent,
            (PointerEventHandler)ShellMenuBarSettingsButton_PointerReleased);
        App.GetService<ControlService>().Stop();
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

    private void ShellMenuBarSettingsButton_PointerEntered(object sender, PointerRoutedEventArgs e)
    {
        AnimatedIcon.SetState((UIElement)sender, "PointerOver");
    }

    private void ShellMenuBarSettingsButton_PointerPressed(object sender, PointerRoutedEventArgs e)
    {
        AnimatedIcon.SetState((UIElement)sender, "Pressed");
    }

    private void ShellMenuBarSettingsButton_PointerReleased(object sender, PointerRoutedEventArgs e)
    {
        AnimatedIcon.SetState((UIElement)sender, "Normal");
    }

    private void ShellMenuBarSettingsButton_PointerExited(object sender, PointerRoutedEventArgs e)
    {
        AnimatedIcon.SetState((UIElement)sender, "Normal");
    }
}