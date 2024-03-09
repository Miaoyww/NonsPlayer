using System.Net;
using System.Net.WebSockets;
using System.Text;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer;

namespace NonsPlayer.Core.Services;

public class ControlFactory
{
    #region Command And Result

    public enum Command
    {
        Play,
        PlayNext,
        PlayPrevious,
        GetMusicInfo,
        GetPlayerState,
        Like,
        VolumeUp,
        VolumeDown,
        Mute,
        SwitchPlayMode,
        SwitchShuffle
    }

    public class Result
    {
        public bool IsFailed { get; set; }

        public string Message { get; set; }

        public static Result ResultFactory(bool isFailed = false, string message = "")
        {
            return new Result
            {
                IsFailed = isFailed,
                Message = message
            };
        }
    }

    #endregion

    private float PreviousVolume;

    public Command ParseCommand(string command)
    {
        switch (command)
        {
            case "play":
                return Command.Play;
            case "play_next":
                return Command.PlayNext;
            case "play_previous":
                return Command.PlayPrevious;
            case "get_music_info":
                return Command.GetMusicInfo;
            case "get_player_state":
                return Command.GetPlayerState;
            case "like":
                return Command.Like;
            default:
                return Command.Play;
        }
    }

    public async Task<Result?> Execute(Command command)
    {
        switch (command)
        {
            case Command.Play:
                Player.Instance.Play();
                return Result.ResultFactory();
            case Command.PlayNext:
                PlayQueue.Instance.PlayNext();
                return Result.ResultFactory();
            case Command.PlayPrevious:
                PlayQueue.Instance.PlayPrevious();
                return Result.ResultFactory();
            case Command.GetMusicInfo:
                return Result.ResultFactory();
            case Command.GetPlayerState:
                return Result.ResultFactory();
            case Command.Like:
                var code = await FavoritePlaylistService.Instance.LikeAsync(PlayQueue.Instance.CurrentMusic.Id);
                if (code != 200)
                {
                    switch (code)
                    {
                        case 301:
                            return new Result
                            {
                                IsFailed = true,
                                Message = "请登录后再试"
                            };
                        case 400:
                            return new Result
                            {
                                IsFailed = true,
                                Message = "请检查网络后再试"
                            };
                        default:
                            return new Result
                            {
                                IsFailed = true,
                                Message = $"出现了错误 {code}"
                            };
                    }
                }

                return Result.ResultFactory();
            case Command.Mute:
                Mute();
                return Result.ResultFactory();
            case Command.SwitchShuffle:
                PlayQueue.Instance.SwitchShuffle();
                return Result.ResultFactory();
            case Command.SwitchPlayMode:
                PlayQueue.Instance.SwitchPlayMode();
                return Result.ResultFactory();
            case Command.VolumeUp:
                Player.Instance.Volume += 5;
                return Result.ResultFactory();
            case Command.VolumeDown:
                Player.Instance.Volume -= 5;
                return Result.ResultFactory();
            
            default:
                return new Result
                {
                    IsFailed = false,
                };
        }
    }

    private void Mute()
    {
        if (Player.Instance.Volume > 0)
        {
            PreviousVolume = Player.Instance.Volume;
            Player.Instance.Volume = 0;
        }
        else
        {
            Player.Instance.Volume = PreviousVolume;
        }
    }
}

public class ControlService
{
    private const string API_VERSION = "v1";
    private HttpListener _httpListener;
    private CancellationTokenSource _cancellationTokenSource;

    private ControlFactory _controlFactory = new();

    public ControlService()
    {
        _httpListener = new HttpListener();
        _httpListener.Prefixes.Add($"http://192.168.101.111:8080/nons/api/{API_VERSION}/");
        _cancellationTokenSource = new CancellationTokenSource();
    }

    public async Task StartAsync()
    {
        _httpListener.Start();
        Console.WriteLine("WebSocket server started.");

        while (!_cancellationTokenSource.Token.IsCancellationRequested)
        {
            var context = await _httpListener.GetContextAsync();
            if (context.Request.IsWebSocketRequest)
            {
                ProcessWebSocketRequest(context);
            }
            else
            {
                context.Response.StatusCode = 400;
                context.Response.Close();
            }
        }
    }

    public void Stop()
    {
        try
        {
            _httpListener.Stop();
            _cancellationTokenSource.Cancel();
            Console.WriteLine("WebSocket server stopped.");
        }
        catch (Exception e)
        {

        }
 
    }

    private async void ProcessWebSocketRequest(HttpListenerContext context)
    {
        var webSocketContext = await context.AcceptWebSocketAsync(null);
        var webSocket = webSocketContext.WebSocket;

        //TODO: 在这里进行用户身份验证

        byte[] buffer = new byte[1024];
        WebSocketReceiveResult result =
            await webSocket.ReceiveAsync(new ArraySegment<byte>(buffer), CancellationToken.None);

        while (!result.CloseStatus.HasValue)
        {
            var message = Encoding.UTF8.GetString(buffer, 0, result.Count);
            var command = _controlFactory.ParseCommand(message);
            await _controlFactory.Execute(command);
            Console.WriteLine($"Received message: {message}");
            result = await webSocket.ReceiveAsync(new ArraySegment<byte>(buffer), CancellationToken.None);

            var replyMessage = "Received your message: " + message;
            var replyBuffer = Encoding.UTF8.GetBytes(replyMessage);
            await webSocket.SendAsync(new ArraySegment<byte>(replyBuffer), WebSocketMessageType.Text, true,
                CancellationToken.None);
        }

        await webSocket.CloseAsync(result.CloseStatus.Value, result.CloseStatusDescription, CancellationToken.None);
    }
}