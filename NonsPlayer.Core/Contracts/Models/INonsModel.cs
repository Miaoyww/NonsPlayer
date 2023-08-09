namespace NonsPlayer.Core.Contracts.Models;

public class INonsModel
{
    public long Id { get; set; }
    public string Name { get; set; }
    public string AvatarUrl { get; set; }
    // public string Type { get; set; }
    public string SmallAvatarUrl => AvatarUrl + "?param=50y50";
    public string MiddleAvatarUrl => AvatarUrl + "?param=200y200";
    public string CacheAvatarId => Id + "_avatar";
    // public string CacheAvatarId => Id + $"_{Type}" + "_avatar";
    public string CacheSmallAvatarId => CacheAvatarId + "_small";
    public string CacheMiddleAvatarId => CacheAvatarId + "_middle";
}