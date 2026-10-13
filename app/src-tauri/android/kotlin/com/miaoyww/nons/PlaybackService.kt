package com.miaoyww.nons

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import androidx.core.app.NotificationCompat

/** Rust owns playback; this service keeps its process and CPU runnable off screen. */
class PlaybackService : Service() {
    private var wakeLock: PowerManager.WakeLock? = null

    override fun onCreate() {
        super.onCreate()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            getSystemService(NotificationManager::class.java).createNotificationChannel(
                NotificationChannel(CHANNEL, "音乐播放", NotificationManager.IMPORTANCE_LOW)
            )
        }
        wakeLock = getSystemService(PowerManager::class.java)
            .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "Nons:playback")
            .apply { setReferenceCounted(false) }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // Do not resurrect a service without a live Rust playback session.
        if (intent == null) {
            stopSelf()
            return START_NOT_STICKY
        }
        val open = PendingIntent.getActivity(
            this, 0, Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        val notification = NotificationCompat.Builder(this, CHANNEL)
            .setSmallIcon(R.drawable.ic_playback_notification)
            .setContentTitle(intent.getStringExtra("title").orEmpty().ifEmpty { "Nons 音乐播放" })
            .setContentText(intent.getStringExtra("artist").orEmpty())
            .setContentIntent(open)
            .setCategory(Notification.CATEGORY_TRANSPORT)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .build()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(NOTIFICATION, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK)
        } else {
            startForeground(NOTIFICATION, notification)
        }
        if (wakeLock?.isHeld == false) wakeLock?.acquire()
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        if (wakeLock?.isHeld == true) wakeLock?.release()
        wakeLock = null
        stopForeground(STOP_FOREGROUND_REMOVE)
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    companion object {
        private const val CHANNEL = "nons-playback"
        private const val NOTIFICATION = 1
    }
}
