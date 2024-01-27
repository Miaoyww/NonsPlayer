namespace NonsPlayer.Helpers;

public class StartUpHelper
{
    public static StartUpHelper Instance { get; } = new();

    public async Task Main()
    {
        await Task.WhenAll();
    }
}