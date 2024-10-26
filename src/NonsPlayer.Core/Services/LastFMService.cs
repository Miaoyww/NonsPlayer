using IF.Lastfm.Core.Api;

namespace NonsPlayer.Core.Services;

public class LastFMService
{
    private const string LastFMAPIKey = "22a44c4543a01040deaf60265a3c30e4";
    private const string LastFMAPISecret = "0a04baf46f4a6f75df9347ba29c488c2";
    public LastfmClient LastfmClient = new(LastFMAPIKey, LastFMAPISecret);

    public static LastFMService Instance { get; } = new LastFMService();

    public LastFMService()
    {
    }
}