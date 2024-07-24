using NonsPlayer.Core.Contracts.Models;

namespace NonsPlayer.Core.Contracts.Models.Nons;

public interface IAccount : INonsModel
{
    string Name { get; set; }
    string Token { get; set; }
    string AvatarUrl { get; set; }
    bool IsLoggedIn { get; set; }
    string CacheAvatarId => Id + "_avatar";
    string CacheSmallAvatarId => CacheAvatarId + "_small";
    string CacheMiddleAvatarId => CacheAvatarId + "_middle";
    string Key { get; set; }

    Task<bool> LoginByToken(string token);
    Task<string> GetAvatarUrl();
    Task<bool> Refresh();
}