namespace NonsPlayer.Core.Api
{
    public class LoginFailed : Exception
    {
        public LoginFailed()
        {
        }

        public LoginFailed(string message) : base(message)
        {
        }

        public override string Message
        {
            get
            {
                return base.Message;
            }
        }
    }
}