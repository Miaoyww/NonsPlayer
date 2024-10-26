using System.Text.Json.Serialization;

namespace NonsPlayer.Updater.Github;

public class GithubMarkdown
{
    [JsonPropertyName("text")] public string Text { get; set; }


    [JsonPropertyName("mode")] public string Mode { get; set; }


    [JsonPropertyName("context")] public string Context { get; set; }
}