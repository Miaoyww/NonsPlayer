﻿using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Account;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IAccountAdapter : ISubAdapter
{
    string Uid { get;  set; }

    string Name { get;  set; }

    string FaceUrl { get;  set; } 
    
    public bool IsLoggedIn
    {
        get
        {
            if (!Token.Equals(string.Empty))
                return true;
            return false;
        }
    }

    public string Token { get; set; }
    bool IsEnable { get; set; }

    Task<Uri> GetQrCode();

    Task<Account> GetAccountAsync(string token);

    Task<string> GetToken(string response);
}