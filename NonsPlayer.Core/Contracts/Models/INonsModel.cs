using System.Text.Json.Serialization;

namespace NonsPlayer.Core.Contracts.Models;

public class INonsModel
{
    [JsonPropertyName("id")] public long Id { get; set; }
    [JsonPropertyName("md5")] public string Md5 { get; set; }
    [JsonPropertyName("name")] public string Name { get; set; }
    [JsonPropertyName("shareUrl")] public string ShareUrl { get; set; }
    [JsonPropertyName("avatarUrl")] public string AvatarUrl { get; set; }

    // public string Type { get; set; }
    [JsonPropertyName("small_avatar_url")] public string SmallAvatarUrl => AvatarUrl + "?param=50y50";

    [JsonPropertyName("siddle_avatar_url")]
    public string MiddleAvatarUrl => AvatarUrl + "?param=200y200";

    [JsonPropertyName("cache_avatar_id")] public string CacheAvatarId => Id + "_avatar";

    // public string CacheAvatarId => Id + $"_{Type}" + "_avatar";
    [JsonPropertyName("cache_small_avatar_id")]
    public string CacheSmallAvatarId => CacheAvatarId + "_small";

    [JsonPropertyName("cache_middle_avatarId")]
    public string CacheMiddleAvatarId => CacheAvatarId + "_middle";
}