using System.Diagnostics;
using CommunityToolkit.Mvvm.ComponentModel;
using F23.StringSimilarity;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Adapters;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

public class SearchViewModel : ObservableRecipient, INavigationAware
{
    private string queryKey;

    public async void OnNavigatedTo(object parameter)
    {
        queryKey = (parameter as string).ToLower();
        await Search(queryKey).ConfigureAwait(false);
    }

    public void OnNavigatedFrom()
    {
    }


    private void QuickSortAlgorithm(List<Tuple<SearchDataType, Tuple<double, JObject>>> data, int low, int high)
    {
        if (low < high)
        {
            var pivotIndex = Partition(data, low, high);
            QuickSortAlgorithm(data, low, pivotIndex - 1);
            QuickSortAlgorithm(data, pivotIndex + 1, high);
        }
    }

    private int Partition(List<Tuple<SearchDataType, Tuple<double, JObject>>> data, int low, int high)
    {
        double pivotValue = data[high].Item2.Item1;
        int i = low - 1;

        for (int j = low; j < high; j++)
        {
            if (data[j].Item2.Item1 < pivotValue)
            {
                i++;
                Swap(data, i, j);
            }
        }

        Swap(data, i + 1, high);
        return i + 1;
    }

    private void Swap(List<Tuple<SearchDataType, Tuple<double, JObject>>> data, int i, int j)
    {
        (data[i], data[j]) = (data[j], data[i]);
    }


    private Tuple<SearchDataType, Tuple<double, JObject>> ParseResult(SearchDataType type, string name,
        JObject originalJObject)
    {
        var l = new Levenshtein();
        return new Tuple<SearchDataType, Tuple<double, JObject>>
        (
            type,
            new Tuple<double, JObject>(l.Distance(name, queryKey.ToLower()), originalJObject)
        );
    }

    public async Task Search(string key)
    {
        var result = await Apis.Search.Default(key, 1, 1, Nons.Instance);
        SearchHelper.Instance.BestMusicResult =
            await MusicAdapters.CreateById(result["result"]["songs"][0]["id"].ToObject<long>());
    }
}