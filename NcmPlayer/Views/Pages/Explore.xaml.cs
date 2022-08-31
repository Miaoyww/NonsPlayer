using NcmPlayer.Resources;
using System.Windows.Controls;

namespace NcmPlayer.Views.Pages
{
    /// <summary>
    /// Explore.xaml 的交互逻辑
    /// </summary>
    public partial class Explore : Page
    {
        public Explore()
        {
            InitializeComponent();
            DataContext = ResEntry.res;
        }
    }
}