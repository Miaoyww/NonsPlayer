namespace NonsPlayer.Core.Services;

public class SavedPlaylistService
{
    public delegate void SavedPlaylistChangedEventHandler();

    public List<string> SavedPlaylists = new();
    public static SavedPlaylistService Instance { get; } = new();

    public event SavedPlaylistChangedEventHandler SavedPlaylistChanged;

    public bool IsLiked(long? id)
    {
        if (SavedPlaylists.Contains(id.ToString())) return true;

        return false;
    }

    public void Init(List<string> savedPlaylistIds)
    {
        SavedPlaylists = savedPlaylistIds;
    }
}