namespace NonsPlayer.Core.Exceptions;

public class PlaylistIdNullException : Exception
{
    public PlaylistIdNullException() : base()
    {
        
    }

    public PlaylistIdNullException(string message) : base(message)
    {
    }

    public PlaylistIdNullException(string message, Exception inner) : base(message, inner)
    {
    }
}