namespace NonsPlayer.Core.Exceptions;

[Serializable]
public class LoginFailureException : Exception
{
    public LoginFailureException() : base()
    {
        
    }

    public LoginFailureException(string message) : base(message)
    {
    }

    public LoginFailureException(string message, Exception inner) : base(message, inner)
    {
    }
}