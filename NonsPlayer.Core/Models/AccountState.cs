using System.Text.Json.Serialization;

namespace NonsPlayer.Core.Models;

public class AccountState
{
    [JsonPropertyName("uid")]
    public string Uid
    {
        get;
        set;
    }

    [JsonPropertyName("name")]
    public string Name
    {
        get;
        set;
    }

    [JsonPropertyName("face_url")]
    public string FaceUrl
    {
        get;
        set;
    }

    [JsonPropertyName("is_logged_in")]
    public bool IsLoggedIn
    {
        get;
        set;
    }
}