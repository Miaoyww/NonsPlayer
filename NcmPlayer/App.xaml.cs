using System.Windows;

namespace NcmPlayer
{
    public partial class App : Application
    {
        private void Application_Startup(object sender, StartupEventArgs e)
        {
            OnloadFunc.Load();
        }

        private void Application_Exit(object sender, ExitEventArgs e)
        {
            OnCloseFunc.Close();
        }
    }
}