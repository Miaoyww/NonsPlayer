using NcmPlayer.Resources;
using System;
using System.Threading;
using System.Windows;
using System.Windows.Controls;

namespace NcmPlayer.Views.Pages
{
    public partial class Home : Page
    {
        public Home()
        {
            InitializeComponent();
            DataContext = ResEntry.res;
        }

        private void Button_Click(object sender, RoutedEventArgs e)
        {
            ThreadPool.QueueUserWorkItem(_ =>
            {
                Dispatcher.BeginInvoke(new Action(() =>
                {
                    CloudMusic.Tool.OpenPlayListDetail(tbox_id.Text);
                }));
            });
        }
    }
}