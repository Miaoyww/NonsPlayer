using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Account;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IAccountAdapter: IAdapter
{
    bool IsEnable { get; set; }
    
    Task<Uri> GetQrCode();
    
    Task<Account> GetAccountAsync(string token);

    Task<string> GetToken(string response);
}