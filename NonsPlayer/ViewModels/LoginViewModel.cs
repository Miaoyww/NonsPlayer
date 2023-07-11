using System.ComponentModel;
using System.Drawing.Imaging;
using System.Runtime.CompilerServices;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Framework;
using NonsPlayer.Framework.Api;
using NonsPlayer.Framework.Resources;
using NonsPlayer.Services;
using QRCoder;

namespace NonsPlayer.ViewModels;

public partial class LoginViewModel : ObservableRecipient, INotifyPropertyChanged
{
    public LoginViewModel()
    {
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }

    private ImageBrush _qrCode;
    private string _key;
    private string _text;

    public ImageBrush QrCode
    {
        get => _qrCode;
        set
        {
            if (_qrCode != value)
            {
                _qrCode = value;
                OnPropertyChanged(nameof(QrCode));
            }
        }
    }

    public string Text
    {
        get => _text;
        set
        {
            if (_text != value)
            {
                _text = value;
                OnPropertyChanged(nameof(Text));
            }
        }
    }


    public async void Init()
    {
        Text = "扫描二维码登录";
        await GetQrCode();
        checkKey();
    }

    public async Task GetQrCode()
    {
        if (_key == null)
        {
            _key = (await Apis.Login.QRCode.Key(timestamp(), Nons.Instance))["unikey"].ToString();
        }

        var qrCodeImage =
            new QRCode(new QRCodeGenerator().CreateQrCode(
                new PayloadGenerator.Url($"http://music.163.com/login?codekey={_key}").ToString(),
                QRCodeGenerator.ECCLevel.M)).GetGraphic(10);
        var ms = new MemoryStream();
        qrCodeImage.Save(ms, ImageFormat.Png);
        ms.Seek(0, SeekOrigin.Begin);
        var result = new BitmapImage();
        result.SetSource(ms.AsRandomAccessStream());
        QrCode = new ImageBrush
        {
            ImageSource = result
        };
    }

    private async Task checkKey()
    {
        // 重复检查二维码状态
        await Task.Run(async () =>
        {
            while (true)
            {
                var result = await Apis.Login.QRCode.Check(_key, Nons.Instance);
                var code = int.Parse(JObject.Parse(result.Content)["code"].ToString());
                if (code == 800)
                {
                    // 二维码过期
                    await GetQrCode();
                }
                else if (code == 802)
                {
                    // 二维码已确认
                    ServiceEntry.DispatcherQueue.TryEnqueue(() =>
                    {
                        Text = "请在手机上确认登录";
                    });
                }
                else if (code == 803)
                {
                    // 二维码登录成功
                    ServiceEntry.DispatcherQueue.TryEnqueue(() =>
                    {
                        Text = "登录成功";
                    });
                    foreach (var cookieItem in result.Cookies)
                    {
                        if (cookieItem.Name == "MUSIC_U")
                        {
                            AccountService.Instance.LoginByToken(cookieItem.Value);
                        }
                    }

                    ServiceEntry.DispatcherQueue.TryEnqueue(() =>
                    {
                        ServiceEntry.NavigationService.NavigateTo(typeof(PersonalCenterViewModel).FullName!);
                    });
                    break;
                }

                await Task.Delay(1000);
            }
        });
    }

    private string timestamp()
    {
        return ((DateTimeOffset)DateTime.Now).ToUnixTimeMilliseconds().ToString();
    }
}