using Microsoft.Win32;
using NcmPlayer.Player;
using NcmPlayer.Resources;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;
using System.Timers;
using System.Windows;
using Wpf.Ui.Appearance;

namespace NcmPlayer
{
    public partial class App : Application
    {
        private void Application_Startup(object sender, StartupEventArgs e)
        {
            OnloadFunc.Load();
        }
    }
}