using Microsoft.UI.Xaml.Media.Imaging;
using Microsoft.UI.Xaml.Media;
using Windows.Storage.Streams;

namespace NonsPlayer.Utils;

public static class ImageUtils
{
    public static async Task<ImageBrush?> GetImageBrushAsyncFromBytes(byte[] cover)
    {
        if (cover == null) return null;
        
        using (var stream = new InMemoryRandomAccessStream())
        {
            // 使用DataWriter将字节数组写入流
            using (DataWriter writer = new DataWriter(stream.GetOutputStreamAt(0)))
            {
                writer.WriteBytes(cover);
                await writer.StoreAsync();
            }

            // 创建BitmapImage并从流中加载图像
            var bitmapImage = new BitmapImage();
            await bitmapImage.SetSourceAsync(stream);
            return new ImageBrush { ImageSource = bitmapImage };
        }
    }

    public static async Task<BitmapImage?> GetBitmapImageFromServer(string imageUrl)
    {
        using (var httpClient = new HttpClient())
        {
            try
            {
                var response = await httpClient.GetAsync(imageUrl);
                response.EnsureSuccessStatusCode();
                var imageBytes = await response.Content.ReadAsByteArrayAsync();
                using (var stream = new InMemoryRandomAccessStream())
                {
                    using (var writer = new DataWriter(stream.GetOutputStreamAt(0)))
                    {
                        writer.WriteBytes(imageBytes);
                        await writer.StoreAsync();
                    }

                    var bitmap = new BitmapImage();
                    await bitmap.SetSourceAsync(stream);
                    return bitmap;
                }
            }
            catch (Exception ex)
            {
                return null;
            }
        }
    }

    public static async Task<IRandomAccessStream?> GetImageStreamFromServer(string imageUrl)
    {
        using var httpClient = new HttpClient();
        try
        {
            var response = await httpClient.GetAsync(imageUrl);
            response.EnsureSuccessStatusCode();
            var imageBytes = await response.Content.ReadAsByteArrayAsync();
            var stream = new InMemoryRandomAccessStream();
            using (var writer = new DataWriter(stream.GetOutputStreamAt(0)))
            {
                writer.WriteBytes(imageBytes);
                await writer.StoreAsync();
            }

            return stream;
        }
        catch (Exception ex)
        {
            return null;
        }
    }
}