using System.Diagnostics;
using CommunityToolkit.Mvvm.ComponentModel;
using F23.StringSimilarity;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.ViewModels;
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
        var (result, elapsed) =
            await Task.WhenAll(Apis.Search.Default(key, 2, 1018, Nons.Instance)).MeasureExecutionTimeAsync();
        Debug.WriteLine($"搜索耗时：{elapsed.TotalMilliseconds}ms");
        var songResult = (JArray) result[0]["result"]["song"]["songs"];
        var artistResult = (JArray) result[0]["result"]["artist"]["artists"];
        var playlistResult = (JArray) result[0]["result"]["playList"]["playLists"];
        var albumResult = (JArray) result[0]["result"]["album"]["albums"];
        var data = new List<Tuple<SearchDataType, Tuple<double, JObject>>>
        {
            ParseResult(SearchDataType.Music, songResult[0]["name"].ToString().ToLower(), (JObject) songResult[0]),
            ParseResult(SearchDataType.Artist, artistResult[0]["name"].ToString().ToLower(), (JObject) artistResult[0]),
            ParseResult(SearchDataType.Playlist, playlistResult[0]["name"].ToString().ToLower(),
                (JObject) playlistResult[0]),
            ParseResult(SearchDataType.Album, albumResult[0]["name"].ToString().ToLower(), (JObject) albumResult[0])
        };
        QuickSortAlgorithm(data, 0, data.Count - 1);
        if (data[0].Item2.Item1 == data[1].Item2.Item1)
        {
            if (data[1].Item1 == SearchDataType.Music)
            {
                (data[1], data[0]) = (data[0], data[1]);
            }
        }

        SearchHelper.Instance.BestMusicResult = await Music.CreateAsync(data[0].Item2.Item2);
    }
}