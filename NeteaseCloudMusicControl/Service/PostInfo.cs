using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace NeteaseCloudMusicControl.Service
{
    public static class PostInfo
    {
        public static void InputLog(params string[] actions)
        {
            string output = $"[{DateTime.Now.Hour}:{DateTime.Now.Minute}:{DateTime.Now.Second}] ";
            foreach (string action in actions)
            {
                output += action + " ";
            }
            CurrentResources.log += output + "\r\n";
        }
    }
}
