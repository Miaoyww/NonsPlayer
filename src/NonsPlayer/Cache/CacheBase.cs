namespace NonsPlayer.Cache;

public class CacheBase
{
    public object? Value;
    private DateTime ExpirationTime;

    public CacheBase(object? value)
    {
        Value = value;
        ExpirationTime = DateTime.Now.AddMinutes(10);
    }

    public bool IsExpired()
    {
        return DateTime.Now > ExpirationTime;
    }
}