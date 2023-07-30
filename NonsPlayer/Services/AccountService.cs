using System.ComponentModel;
using System.Runtime.CompilerServices;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Account;
using NonsPlayer.Core.Api;
using NonsPlayer.Helpers;
using NonsPlayer.Heplers;
using NonsPlayer.Models;

namespace NonsPlayer.Services;

public class AccountService
{
    public static AccountService Instance
    {
        get;
    } = new();

    public AccountService()
    {
        Account.Instance.AccountInitializedHandle += OnAccountInitialized;
    }

    public void UpdateInfo()
    {
        AccountState.Instance.Uid = Account.Instance.Uid;
        AccountState.Instance.Name = Account.Instance.Name;
        AccountState.Instance.FaceUrl = Account.Instance.FaceUrl;
    }
    public void OnAccountInitialized()
    {
        UpdateInfo();
        UserPlaylistHelper.Instance.Init();
    }

}