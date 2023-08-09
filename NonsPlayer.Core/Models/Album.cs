using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Contracts.Models;

namespace NonsPlayer.Core.Models;

public class Album : INonsModel
{
    [JsonPropertyName("create_date")] public DateTime CreateDate;
    [JsonPropertyName("description")] public string Description;
    public List<Music> Musics { get; set; }
    public List<Artist> Artists { get; set; }
    [JsonPropertyName("artists_name")] public string ArtistsName => string.Join("/", Artists.Select(x => x.Name));
    public int CollectionCount { set; get; }
    public int TrackCount { get; set; }
}