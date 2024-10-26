using NonsPlayer.Updater.Github;
using System.Text.Json.Serialization;

namespace NonsPlayer.Updater.Metadata;

[JsonSerializable(typeof(ReleaseVersion))]
[JsonSerializable(typeof(GithubRelease))]
[JsonSerializable(typeof(List<GithubRelease>))]
internal partial class MetadataJsonContext : JsonSerializerContext
{

}