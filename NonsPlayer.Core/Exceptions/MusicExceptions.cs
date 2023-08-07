namespace NonsPlayer.Core.Exceptions;

public class MusicIdNullException : Exception
{
    public MusicIdNullException()
    {
    }

    public MusicIdNullException(string message) : base(message)
    {
    }

    public MusicIdNullException(string message, Exception inner) : base(message, inner)
    {
    }
}