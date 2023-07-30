namespace NonsPlayer.Cache;

public class CacheItem<T>
{
    public T Data
    {
        get;
        set;
    }

    public DateTime ExpirationTime
    {
        get;
        set;
    }

    public CacheItem(DateTime? ExpirationTime = null)
    {
        this.ExpirationTime = ExpirationTime ?? DateTime.Now.AddMinutes(1);
    }
}