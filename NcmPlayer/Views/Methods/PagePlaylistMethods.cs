using System;
using System.Windows.Input;

namespace NcmPlayer.Views.Methods
{
    internal class PagePlaylistMethods : ICommand
    {
        private readonly Action<object> execAction;
        private readonly Func<object, bool> changeFunc;

        public PagePlaylistMethods(Action<object> ecA, Func<object, bool> cFunc)
        {
            execAction = ecA;
            changeFunc = cFunc;
        }

        public event EventHandler? CanExecuteChanged;

        public bool CanExecute(object? parameter)
        {
            return changeFunc.Invoke(parameter);
        }

        public void Execute(object? parameter)
        {
            execAction(parameter);
        }
    }
}