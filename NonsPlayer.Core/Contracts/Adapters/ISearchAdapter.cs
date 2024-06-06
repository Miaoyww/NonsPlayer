using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface ISearchAdapter : ISubAdapter
{
    Task<SearchResult> SearchAsync(string keywords);
}