using NonsPlayer.Core.Account;
using NonsPlayer.Helpers;
using NonsPlayer.Heplers;
using NonsPlayer.Models;

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
            AccountState.Instance.Uid = Account.Instance.Uid;
            AccountState.Instance.Name = Account.Instance.Name;
            AccountState.Instance.FaceUrl = Account.Instance.FaceUrl;
        });
    }

    public void OnAccountInitialized()
    {
        UpdateInfo();
        UserPlaylistHelper.Instance.Init();
    }
}