using System.Text.Json.Serialization;

namespace NonsPlayer.Core.Models;

public class PlayerState
{
    [JsonPropertyName("position")]
    public TimeSpan Position
    {
        get;
        set;
    }

    [JsonPropertyName("current_music")]
    public Music CurrentMusic
    {
        get;
        set;
    }
    
    [JsonPropertyName("volume")]
    public float Volume
    {
        get;
        set;
    }
}