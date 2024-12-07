using System.Diagnostics;
using System.Drawing.Imaging;
using System.Reflection;
using Windows.Storage;
using Windows.Storage.AccessCache;
using Windows.Storage.Pickers;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using LiteDB;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NAudio.Wave;
using NAudio.Wave.SampleProviders;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.AMLL.Parsers;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Services;
using NonsPlayer.DataBase;
using NonsPlayer.DataBase.Models;

namespace NonsPlayer.ViewModels;

public partial class TestViewModel : ObservableObject
{
    [RelayCommand]
    public async Task Test(string data)
    {
        using(var db = new LiteDatabase(Path.Join(ConfigManager.Instance.Settings.Data, "test.db")))
        {
            var col = db.GetCollection<DbFolderModel>("folders");
            
            var customer = new DbFolderModel()
            { 
                Path = "TEST",
                Songs = new List<DbSongModel>()
                {
                    new DbSongModel()
                    {
                        Path = "TEST",
                        Title = "TEST",
                        Track = 1,
                        Album = "TEST",
                        Artist = "TEST",
                        Duration = 1,
                        Bitrate = 1,
                        Created = 1,
                        Rate = 1
                    }
                }
            };
            var customer2 = new DbFolderModel()
            {
                Path = "JEST",
                Songs = new List<DbSongModel>()
                {
                    new DbSongModel()
                    {
                        Path = "JEST",
                        Title = "JEST",
                        Track = 1,
                        Album = "TEST",
                        Artist = "TEST",
                        Duration = 1,
                        Bitrate = 1,
                        Created = 1,
                        Rate = 1
                    }
                }
            };

            col.Insert(customer);
            col.Insert(customer2);

            // Index document using document Name property
            col.EnsureIndex(x => x.Path);
	
            // Use LINQ to query documents (filter, sort, transform)
            var results = col.Query()
                .Where(x => x.Path.StartsWith("J"))
                .OrderBy(x => x.Path)
                .Select(x => new { x.Path, Songs = x.Songs })
                .Limit(10)
                .ToList();
            Debug.WriteLine(results[0].Songs[0].Title);
        }
    }
}