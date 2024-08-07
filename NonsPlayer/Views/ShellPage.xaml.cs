﻿using Windows.System;
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
using Windows.UI.WindowManagement;
using Windows.Win32;
using Windows.Win32.Foundation;
using AppWindowTitleBar = Microsoft.UI.Windowing.AppWindowTitleBar;

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

        // TODO: Set the title bar icon by updating /Assets/WindowIcon.ico.
        // A custom title bar is required for full window theme and Mica support.
        // https://docs.microsoft.com/windows/apps/develop/title-bar?tabs=winui3#full-customization
        App.MainWindow.ExtendsContentIntoTitleBar = true;
        App.MainWindow.SetTitleBar(AppTitleBar);
        App.MainWindow.Activated += MainWindow_Activated;
        SetDragRectangles(new RectInt32(0, 0, 100000, (int)(48 * UIScale)));
        AppTitleBarText.Text = "AppDisplayName".GetLocalized();
        App.MainWindow.AppWindow.TitleBar.IconShowOptions = IconShowOptions.HideIconAndSystemMenu;
        App.MainWindow.AppWindow.TitleBar.PreferredHeightOption = TitleBarHeightOption.Tall;
        PlayerBar.OnPlayQueueBarOpenHandler += OnOpenPlayQueueButton_Click;
        ExceptionService.Instance.ExceptionThrew += OnExceptionThrew;
    }

    public void SetDragRectangles(params RectInt32[] value)
    {
        if (AppWindowTitleBar.IsCustomizationSupported() && App.MainWindow.AppWindow.TitleBar.ExtendsContentIntoTitleBar == true)
        {
            App.MainWindow.AppWindow.TitleBar.SetDragRectangles(value);
        }
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
}