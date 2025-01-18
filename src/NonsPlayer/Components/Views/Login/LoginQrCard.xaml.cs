using System.Drawing.Imaging;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Nons;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using QRCoder;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class LoginQrCard : UserControl
{
    [ObservableProperty] private ImageBrush qrCode;
    [ObservableProperty] private string qrCodeState;
    private IAccount account;
    private IAdapter adapter;
    private CancellationTokenSource tokenSource;
    private CancellationToken cancellationToken;
    public LoginQrCard()
    {
        ViewModel = App.GetService<LoginCardViewModel>();
        InitializeComponent();
        QrCodeState = "QrCodeState_Waiting".GetLocalized();
        tokenSource = new();
        cancellationToken = tokenSource.Token;
    }
    public LoginCardViewModel ViewModel { get; }

    public IAdapter Adapter
    {
        set
        {
            adapter = value;
            account = adapter.Account.GetAccount();
            Init();
        }
    }

    private async void Init()
    {
        if (account != null)
        {
            if (account.IsLoggedIn)
            {
                await RefreshAccountInfo().ConfigureAwait(false);
            }
            else
            { 
                await GetQrCode();
                await CheckKey().ConfigureAwait(false);
            }
        }
        
    }

    private async Task GetQrCode()
    {
        var data = await adapter.Account.GetQrCodeUrlAsync();
        if (data.Item1 == null) return;
        account.Key = data.Item2;
        var qrCodeImage = new QRCode(new QRCodeGenerator().CreateQrCode(
            new PayloadGenerator.Url(data.Item1.ToString()).ToString(),
            QRCodeGenerator.ECCLevel.M)).GetGraphic(10);
        var ms = new MemoryStream();
        qrCodeImage.Save(ms, ImageFormat.Png);
        ms.Seek(0, SeekOrigin.Begin);
        var result = new BitmapImage();
        result.SetSource(ms.AsRandomAccessStream());

        DispatcherQueue.TryEnqueue(() =>
        {
            QrCode = new ImageBrush
            {
                ImageSource = result
            };
            QrCodeState = "QrCodeState_Waiting".GetLocalized();
        });
        
    }

    private async Task CheckKey()
    {
        // ÖØ¸´¼ì²é¶þÎ¬Âë×´Ì¬
        await Task.Run(async () =>
        {
            while (!cancellationToken.IsCancellationRequested)
            {
                var result = await adapter.Account.CheckLoginAsync(account.Key);
                if (result.Status == QrCodeStatus.Timeout)
                {
                    await GetQrCode();
                }
                else if (result.Status == QrCodeStatus.Scanned)
                {
                    DispatcherQueue.TryEnqueue(() => { QrCodeState = "QrCodeState_Scanned".GetLocalized(); });
                }
                else if (result.Status == QrCodeStatus.Confirmed)
                {
                    // ¶þÎ¬ÂëµÇÂ¼³É¹¦
                    DispatcherQueue.TryEnqueue(() => { QrCodeState = "QrCodeState_Done".GetLocalized(); });
                    account = result.Account;
                    ConfigManager.Instance.Settings.AdapterAccountTokens.Add(adapter.GetMetadata().Slug, account.Token);
                    ConfigManager.Instance.Save();
                    break;
                }
        
                await Task.Delay(1000);
            }
        });
    }

    private async Task RefreshAccountInfo()
    {

    }

    private void LoginQrCard_OnUnloaded(object sender, RoutedEventArgs e)
    {
        tokenSource.Cancel();
    }
}