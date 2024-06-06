using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface ISearchAdapter: ISubAdapter
{
    Task SearchAsync(string keyword, int limit = 20);
}