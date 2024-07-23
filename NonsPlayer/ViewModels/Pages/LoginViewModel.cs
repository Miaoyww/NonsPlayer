using System.Drawing.Imaging;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Nons;
using NonsPlayer.Helpers;
using QRCoder;

namespace NonsPlayer.ViewModels;

public partial class LoginViewModel : ObservableObject
{
    [ObservableProperty] private string key;
    [ObservableProperty] private ImageBrush qrCode;
    [ObservableProperty] private string text;

    public async void Init()
    {
        Text = "扫描二维码登录";
        await GetQrCode();
        checkKey();
    }

    public async Task GetQrCode()
    {
        // if (key == null)
        //     key = (await Apis.Login.QRCode.Key(getCurrentTimestamp(), NonsCore.Instance))["unikey"].ToString();
        //
        // var qrCodeImage =
        //     new QRCode(new QRCodeGenerator().CreateQrCode(
        //         new PayloadGenerator.Url($"http://music.163.com/login?codekey={key}").ToString(),
        //         QRCodeGenerator.ECCLevel.M)).GetGraphic(10);
        // var ms = new MemoryStream();
        // qrCodeImage.Save(ms, ImageFormat.Png);
        // ms.Seek(0, SeekOrigin.Begin);
        // var result = new BitmapImage();
        // result.SetSource(ms.AsRandomAccessStream());
        // QrCode = new ImageBrush
        // {
        //     ImageSource = result
        // };
    }

    private async Task checkKey()
    {
        // // 重复检查二维码状态
        // await Task.Run(async () =>
        // {
        //     while (true)
        //     {
        //         var result = await Apis.Login.QRCode.Check(key, NonsCore.Instance);
        //         var code = int.Parse(JObject.Parse(result.Content)["code"].ToString());
        //         if (code == 800)
        //         {
        //             // 二维码过期
        //             await GetQrCode();
        //         }
        //         else if (code == 802)
        //         {
        //             // 二维码已确认
        //             ServiceHelper.DispatcherQueue.TryEnqueue(() => { Text = "请在手机上确认登录"; });
        //         }
        //         else if (code == 803)
        //         {
        //             // 二维码登录成功
        //             ServiceHelper.DispatcherQueue.TryEnqueue(() => { Text = "登录成功"; });
        //             foreach (var cookieItem in result.Cookies)
        //                 if (cookieItem.Name == "MUSIC_U")
        //                     Account.Instance.LoginByToken(cookieItem.Value);
        //
        //             ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        //             {
        //                 ServiceHelper.NavigationService.NavigateTo(typeof(PersonalCenterViewModel).FullName!);
        //             });
        //             break;
        //         }
        //
        //         await Task.Delay(1000);
        //     }
        // });
    }

    private string getCurrentTimestamp()
    {
        return ((DateTimeOffset)DateTime.Now).ToUnixTimeMilliseconds().ToString();
    }
}