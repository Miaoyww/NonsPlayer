using System.Text.Json.Serialization;


namespace NonsPlayer.Core.Nons;

public class Account
{
    //TODO: 本地账号
    public delegate void AccountInitialized();

    private string _machineCode = string.Empty;
    private string _tokenMd5 = string.Empty;

    public AccountInitialized AccountInitializedHandle;
    [JsonPropertyName("name")] public string Name { get; private set; } = "NonsPlayer";

    [JsonPropertyName("face_url")]
    public string FaceUrl { get; private set; } = "ms-appdata:///Assets/UnKnowResource.png";

    public static Account Instance { get; } = new();
}