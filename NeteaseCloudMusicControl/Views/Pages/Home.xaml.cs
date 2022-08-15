using Microsoft.Win32;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Navigation;
using System.Windows.Shapes;
using NeteaseCloudMusicControl.Views.Methods;
using System.Timers;

namespace NeteaseCloudMusicControl.Views.Pages
{
    public partial class Home : Page
    {
        public Home()
        {
            InitializeComponent();
;
        }

        private void Button_Click(object sender, RoutedEventArgs e)
        {
            CurrentResources.soundPath = tbox_url.Text;
            PlayerMethods.RePlay();
        }
    }
}
