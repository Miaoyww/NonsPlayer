using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Nons.Account;
using NonsPlayer.Helpers;
using NonsPlayer.Heplers;
using NonsPlayer.Models;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Services;

public class AccountService
{
    public AccountService()
    {
        Account.Instance.AccountInitializedHandle += OnAccountInitialized;
    }

    public static AccountService Instance { get; } = new();

    public void UpdateInfo()
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            AccountStateModel.Instance.Uid = Account.Instance.Uid;
            AccountStateModel.Instance.Name = Account.Instance.Name;
            AccountStateModel.Instance.FaceUrl = Account.Instance.FaceUrl;
        });
    }

    public void OnAccountInitialized()
    {
        UpdateInfo();
        UserPlaylistHelper.Instance.Init();
    }
}