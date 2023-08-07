namespace NonsPlayer.Core.Contracts.Models;

public class INonsModel
{
    public long? Id { get; set; }
    public string Name { get; set; }
    public string AvatarUrl { get; set; }
    public string SmallAvatarUrl => AvatarUrl + "?param=50y50";
    public string MiddleAvatarUrl => AvatarUrl + "?param=200y200";
    public string CacheAvatarId => AvatarUrl + "_avatar";
    public string CacheSmallAvatarId => AvatarUrl + "_small_avatar";
    public string CacheMiddleAvatarId => AvatarUrl + "_middle_avatar";
}