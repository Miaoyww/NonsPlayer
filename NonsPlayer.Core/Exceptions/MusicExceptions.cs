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

public class MusicUrlNullException : Exception
{
    public MusicUrlNullException()
    {
    }

    public MusicUrlNullException(string message) : base(message)
    {
    }

    public MusicUrlNullException(string message, Exception inner) : base(message, inner)
    {
    }
}