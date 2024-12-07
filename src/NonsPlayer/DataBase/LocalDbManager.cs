using LiteDB;
using NonsPlayer.Core.Services;
using NonsPlayer.DataBase.Models;

namespace NonsPlayer.DataBase;

public class LocalDbManager : IDisposable
{
    private static readonly Lazy<LocalDbManager> _instance =
        new Lazy<LocalDbManager>(() => new LocalDbManager());

    public static LocalDbManager Instance => _instance.Value;
    private LiteDatabase? _database;

    private const string CollectionName = "folders";

    // 数据库路径A
    private readonly string _databasePath;


    public LocalDbManager()
    {
        _databasePath = Path.Join(ConfigManager.Instance.Settings.Data, "local_songs.db");
        _database = new LiteDatabase(_databasePath);
    }

    public static bool UpdateFolder(DbFolderModel folder)
    {
        var db = Instance.GetDatabase();
        var collection = db.GetCollection<DbFolderModel>(CollectionName);

        // 确保 Id 唯一（LiteDB 默认使用 Id 作为主键）
        collection.EnsureIndex(x => x.Id);

        // 调用 Update 方法更新数据
        return collection.Upsert(folder);
    }

    public static ILiteCollection<DbFolderModel> GetFolders()
    {
        return Instance.GetDatabase().GetCollection<DbFolderModel>(CollectionName);
    }

    public LiteDatabase GetDatabase()
    {
        if (_database == null)
        {
            _database = new LiteDatabase(_databasePath);
        }

        return _database;
    }

    public void Dispose()
    {
        _database?.Dispose();
        _database = null;
    }
}