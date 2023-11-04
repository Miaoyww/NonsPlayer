namespace NonsPlayer.Core.Exceptions;

public class SearchExceptions: Exception
{
    public SearchExceptions()
    {
    }

    public SearchExceptions(string message) : base(message)
    {
    }

    public SearchExceptions(string message, Exception inner) : base(message, inner)
    {
    }
}