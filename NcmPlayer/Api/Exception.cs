using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace NcmApi
{

    public class LoginFailed : Exception
    {
        public LoginFailed() { }
        public LoginFailed(string message) : base(message) { }
        public override string Message
        {
            get
            {
                return base.Message;
            }
        }
    }
}
