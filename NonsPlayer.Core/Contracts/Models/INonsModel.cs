using System.Text.Json.Serialization;

namespace NonsPlayer.Core.Contracts.Models;

public class INonsModel
{
    [JsonPropertyName("id")] public long Id { get; set; }
    [JsonPropertyName("md5")] public string Md5 { get; set; }
    [JsonPropertyName("name")] public string Name { get; set; }
    [JsonPropertyName("shareUrl")] public string ShareUrl { get; set; }
    [JsonPropertyName("avatarUrl")] public string AvatarUrl { get; set; }
    [JsonIgnore] public string SmallAvatarUrl => AvatarUrl + "?param=50y50";
    [JsonIgnore] public string MiddleAvatarUrl => AvatarUrl + "?param=200y200";
    [JsonIgnore] public string CacheAvatarId => Id + "_avatar";
    [JsonIgnore] public string CacheSmallAvatarId => CacheAvatarId + "_small";
    [JsonIgnore] public string CacheMiddleAvatarId => CacheAvatarId + "_middle";
}