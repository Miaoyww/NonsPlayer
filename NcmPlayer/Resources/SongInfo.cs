using System;
using System.ComponentModel;
using System.IO;
using System.Windows.Media;
using System.Windows.Media.Imaging;

namespace NcmPlayer.Resources
{
    public class SongInfo : INotifyPropertyChanged
    {
        public event PropertyChangedEventHandler? PropertyChanged = delegate { };


        private string filePath = string.Empty; // 当前播放音乐的路径
        private bool isPlaying = false;
        private int volume;// 当前的音量
        private string? name = string.Empty;
        private string? artists = string.Empty; // 当前播放音乐的音乐家们, 以后可以使用JObject
        private string albumCoverUrl;
        private string albumId;
        private string lrcString = string.Empty;
        private Stream? coverStream;  // 当前播放音乐的封面流
        private ImageBrush? coverBrush;  // 当前播放音乐的封面Brush
        private TimeSpan position;
        private string postionString = string.Empty; // 当前播放音乐的进度
        private TimeSpan durationTime;
        private string durationTimeString = string.Empty;

        public string LrcString
        {
            get
            {
                return lrcString;
            }
            set
            {
                lrcString = value;
            }
        }

        public string FilePath
        {
            get => filePath;
            set => filePath = value;
        }

        public bool IsPlaying
        {
            set
            {
                isPlaying = value;
                PropertyChanged(this, new PropertyChangedEventArgs("IsPlaying"));
            }
            get
            {
                return isPlaying;
            }
        }

        public int Volume
        {
            set
            {
                volume = value;
                MusicPlayer.Volume((double)value / 100);
                PropertyChanged(this, new PropertyChangedEventArgs("Volume"));
            }
            get
            {
                return volume;
            }
        }

        public string Name
        {
            set
            {
                name = value;
                PropertyChanged(this, new PropertyChangedEventArgs("Name"));
            }
            get
            {
                if (name.Equals(string.Empty))
                {
                    return "当前未播放音乐";
                }
                return name;
            }
        }

        public string Artists
        {
            set
            {
                artists = value;
                PropertyChanged(this, new PropertyChangedEventArgs("Artists"));
            }
            get
            {
                if (artists.Equals(string.Empty))
                {
                    return "无";
                }
                return artists;
            }
        }

        public ImageBrush? Cover(Stream? stream = null)
        {
            if (stream != null)
            {
                BitmapImage image = new();
                image.BeginInit();
                image.StreamSource = stream;
                image.EndInit();
                ImageBrush brush = new();
                brush.ImageSource = image;
                CoverStream = stream;
                CoverBrush =  brush;
                return brush;
            }
            else
            {
                return coverBrush;
            }
        }

        public Stream? CoverStream
        {
            set
            {
                coverStream = value;
                PropertyChanged(this, new PropertyChangedEventArgs("CoverStream"));
            }
            get
            {
                Stream stream = new MemoryStream();
                coverStream.CopyTo(stream);
                return coverStream;
            }
        }

        public ImageBrush CoverBrush
        {
            set
            {
                coverBrush = value;
                PropertyChanged(this, new PropertyChangedEventArgs("CoverBrush"));
            }
            get
            {
                if (coverBrush != null)
                {
                    return coverBrush;
                }
                else
                {
                    return new ImageBrush(new BitmapImage(new Uri("pack://application:,,,/Assets/BackGround.png")));
                }
            }
        }

        public string AlbumCoverUrl
        {
            get
            {
                return albumCoverUrl;
            }
            set
            {
                albumCoverUrl = value;
            }
        }

        public string AlbumId
        {
            get
            {
                return albumId;
            }
            set
            {
                albumId = value;
            }
        }

        public string PostionString
        {
            set
            {
                TimeSpan timespan = TimeSpan.Parse(value);
                int min = timespan.Minutes;
                int sec = timespan.Seconds;
                postionString = timespan.ToString(@"mm\:ss");
                PropertyChanged(this, new PropertyChangedEventArgs("PostionString"));
            }
            get
            {
                if (!postionString.Equals(string.Empty))
                {
                    return postionString;
                }
                else
                {
                    return "00:00";
                }
            }
        }

        public double PostionDouble
        {
            set
            {
                PropertyChanged(this, new PropertyChangedEventArgs("PostionDouble"));
            }
            get
            {
                return position.TotalSeconds;
            }
        }
        public TimeSpan Postion
        {
            set
            {
                PostionString = value.ToString();
                position = value;
                PostionDouble = value.TotalSeconds;
                PropertyChanged(this, new PropertyChangedEventArgs("Postion"));
            }
            get
            {
                return position;
            }
        }

        public string DurationTimeString
        {
            set
            {
                TimeSpan timespan = TimeSpan.Parse(value);
                int min = timespan.Minutes;
                int sec = timespan.Seconds;
                durationTimeString = timespan.ToString(@"mm\:ss");
                PropertyChanged(this, new PropertyChangedEventArgs("DurationTimeString"));
            }
            get
            {
                if (!durationTimeString.Equals(string.Empty))
                {
                    return durationTimeString;
                }
                else
                {
                    return "00:00";
                }
            }
        }

        public double DurationTimeDouble
        {
            set
            {
                PropertyChanged(this, new PropertyChangedEventArgs("DurationTimeDouble"));
            }
            get
            {
                return durationTime.TotalSeconds;
            }
        }

        public TimeSpan DurationTime
        {
            set
            {
                DurationTimeString = value.ToString();
                durationTime = value;
                DurationTimeDouble = value.TotalSeconds;
                PropertyChanged(this, new PropertyChangedEventArgs("DurationTime"));
            }
            get
            {
                return durationTime;
            }
        }
    }
}