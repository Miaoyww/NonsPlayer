namespace NonsPlayer.Core.Utils;

public static class LocalUtils

{
    public static bool IsMusic(string path)
    {
        string extension = Path.GetExtension(path);
        if (string.IsNullOrEmpty(extension)) return false;
        string[] songEx = [".wav", ".flac", ".ape", ".m4a", ".caf", ".wv", ".mp3", ".aac", ".ogg", ".opus"];
        foreach (string s in songEx)
        {
            if (extension.Equals(s)) return true;
        }

        return false;
    }
    
}