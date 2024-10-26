using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Runtime.InteropServices;
using System.Text.Json;
using NonsPlayer.Updater.Github;
using NonsPlayer.Updater.Metadata;

namespace NonsPlayer.Updater.Update;

public class UpdateClient
{
    private const string API_PREFIX_GITHUB = "https://raw.githubusercontent.com/Miaoyww/NonsPlayer/metadata";

    private const string API_VERSION = "v1";

    private readonly HttpClient _httpClient;

    private readonly string API_PREFIX = API_PREFIX_GITHUB;

    public UpdateClient(HttpClient? httpClient = null)
    {
        _httpClient = httpClient ??
                      new HttpClient(new HttpClientHandler { AutomaticDecompression = DecompressionMethods.All })
                          { DefaultRequestVersion = HttpVersion.Version20 };
    }

    public bool IsUpdated { get; set; }


    private string GetUrl(string suffix)
    {
        return $"{API_PREFIX}/{API_VERSION}/{suffix}";
    }


    public async Task<ReleaseVersion> GetLatestVersionAsync(bool isPrerelease, Architecture architecture,
        CancellationToken cancellationToken = default)
    {
#if DEV
        isPrerelease = true;
#endif
        var name = (isPrerelease, architecture) switch
        {
            (false, Architecture.X64) => "version_stable_x64.json",
            (true, Architecture.X64) => "version_preview_x64.json",
            (false, Architecture.Arm64) => "version_stable_arm64.json",
            (true, Architecture.Arm64) => "version_preview_arm64.json",
            _ => throw new PlatformNotSupportedException($"{architecture} is not supported.")
        };
        var url = GetUrl(name);
        return await ParseResponse<ReleaseVersion>(url, cancellationToken);
    }

    public async Task<GithubRelease?> GetGithubReleaseAsync(string tag, CancellationToken cancellationToken = default)
    {
        var url = $"https://api.github.com/repos/Miaoyww/NonsPlayer/releases/tags/{tag}";
        return await ParseResponse<GithubRelease>(url, cancellationToken);
    }

    private async Task<T> ParseResponse<T>(string url, CancellationToken cancellationToken = default) where T : class
    {
        var res =
            await _httpClient.GetFromJsonAsync(url, typeof(T), MetadataJsonContext.Default, cancellationToken) as T;
        if (res == null)
        {
            throw new NullReferenceException($"尝试ParseResponse失败,res为空.URL= {url}");
        }
        else
        {
            return res;
        }
    }

    public async Task<string> RenderGithubMarkdownAsync(string markdown, CancellationToken cancellationToken = default)
    {
        const string url = "https://api.github.com/markdown";
        var request = new GithubMarkdown
        {
            Text = markdown,
            Mode = "gfm",
            Context = "Miaoyww/NonsPlayer"
        };
        var options = new JsonSerializerOptions { WriteIndented = true };
        var content =
            new StringContent(
                JsonSerializer.Serialize(request, typeof(GithubMarkdown), options),
                new MediaTypeHeaderValue("application/json"));
        var response = await _httpClient.PostAsync(url, content, cancellationToken);
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadAsStringAsync();
    }
}