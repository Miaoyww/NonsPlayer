using System.Diagnostics;
using Microsoft.UI.Xaml;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using WinUIEx.Messaging;

namespace NonsPlayer;

public sealed partial class MainWindow : WindowEx
{
    private WindowMessageMonitor monitor;

    public MainWindow()
    {
        InitializeComponent();
        Content = null;
        Title = "AppDisplayName".GetLocalized();
        monitor = new WindowMessageMonitor(this);
        monitor.WindowMessageReceived += MonitorOnWindowMessageReceived;
        KeyHookService.Instance.Init(this.GetWindowHandle());
    }

    private void MonitorOnWindowMessageReceived(object? sender, WindowMessageEventArgs e)
    {
        if (e.Message.MessageId == 0x0312)
        {
            KeyHookService.Instance.OnHotKey((int)e.Message.WParam);
        }
    }

    private void MainWindow_OnActivated(object sender, WindowActivatedEventArgs args)
    {
    }
}