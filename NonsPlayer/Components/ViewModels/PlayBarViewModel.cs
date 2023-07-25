using System.ComponentModel;
using System.Runtime.CompilerServices;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Heplers;
using NonsPlayer.Resources;

namespace NonsPlayer.Components.ViewModels
{
    public class PlayerBarViewModel : INotifyPropertyChanged
    {

        public GlobalMusicState GlobalMusicState => GlobalMusicState.Instance;
        public UserPlaylistHelper UserPlaylistHelper => UserPlaylistHelper.Instance;

        public PlayerBarViewModel()
        {
            GlobalMusicState.Instance.Volume = double.Parse(RegHelper.Instance.Get(RegHelper.Regs.Volume, 0.0).ToString());
        }
        public void Init()
        {

        }

        public event PropertyChangedEventHandler? PropertyChanged;

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
}