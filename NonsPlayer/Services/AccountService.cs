using NonsPlayer.Components.ViewModels;
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
            AccountStateViewModel.Instance.Uid = Account.Instance.Uid;
            AccountStateViewModel.Instance.Name = Account.Instance.Name;
            AccountStateViewModel.Instance.FaceUrl = Account.Instance.FaceUrl;
        });
    }

    public void OnAccountInitialized()
    {
        UpdateInfo();
        UserPlaylistHelper.Instance.Init();
    }
}