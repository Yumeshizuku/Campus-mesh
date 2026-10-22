package org.campusmesh

import android.app.*
import android.content.Intent
import android.os.*
import androidx.core.app.NotificationCompat

class MeshService : Service() {
    private var ble: BleMesh? = null
    private var direct: WifiDirectMesh? = null
    private val timer = Handler(Looper.getMainLooper())
    private var lastIncoming = mutableSetOf<String>()
    private var first = true
    private val poll =
        object : Runnable {
            override fun run() {
                runCatching {
                    val history =
                        Native.request(org.json.JSONObject().put("op", "state"))
                            .getJSONObject("data")
                            .getJSONArray("history")
                    for (i in 0 until history.length()) {
                        val m = history.getJSONObject(i)
                        if (
                            m.getBoolean("incoming") &&
                                lastIncoming.add(m.getString("id")) &&
                                !first
                        ) {
                            val n =
                                NotificationCompat.Builder(this@MeshService, "messages")
                                    .setSmallIcon(android.R.drawable.ic_dialog_email)
                                    .setContentTitle("Campus Mesh · 新消息")
                                    .setContentText("收到一条离线消息，打开应用查看")
                                    .setContentIntent(openIntent())
                                    .setAutoCancel(true)
                                    .build()
                            if (
                                Build.VERSION.SDK_INT < 33 ||
                                    checkSelfPermission(
                                        android.Manifest.permission.POST_NOTIFICATIONS
                                    ) == android.content.pm.PackageManager.PERMISSION_GRANTED
                            )
                                getSystemService(NotificationManager::class.java).notify(2, n)
                        }
                    }
                    first = false
                }
                timer.postDelayed(this, 3000)
            }
        }

    private fun openIntent() =
        PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE,
        )

    override fun onCreate() {
        super.onCreate()
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel("mesh", "离线连接服务", NotificationManager.IMPORTANCE_LOW)
        )
        manager.createNotificationChannel(
            NotificationChannel("messages", "离线消息", NotificationManager.IMPORTANCE_DEFAULT)
        )
        startForeground(
            1,
            NotificationCompat.Builder(this, "mesh")
                .setSmallIcon(android.R.drawable.stat_notify_sync)
                .setContentTitle("Campus Mesh")
                .setContentText("局域网通信运行中 · 点击打开")
                .setContentIntent(openIntent())
                .build(),
        )
        try {
            Native.start(
                filesDir.resolve("mesh.json").path,
                getSharedPreferences("mesh", 0).getString("nickname", "同学")!!,
            )
            timer.post(poll)
        } catch (e: Exception) {
            android.widget.Toast.makeText(this, e.message, android.widget.Toast.LENGTH_LONG).show()
            stopSelf()
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == "BLE_ON")
            runCatching {
                    if (ble == null) ble = BleMesh(this)
                    ble?.start()
                }
                .onFailure {
                    android.widget.Toast.makeText(
                            this,
                            it.message,
                            android.widget.Toast.LENGTH_LONG,
                        )
                        .show()
                }
        if (intent?.action == "BLE_OFF") {
            ble?.stop()
            ble = null
        }
        if (intent?.action == "P2P_DISCOVER")
            runCatching {
                    if (direct == null) direct = WifiDirectMesh(this)
                    direct?.discover()
                }
                .onFailure {
                    android.widget.Toast.makeText(
                            this,
                            it.message,
                            android.widget.Toast.LENGTH_LONG,
                        )
                        .show()
                }
        if (intent?.action == "P2P_CONNECT")
            intent.getStringExtra("address")?.let { direct?.connect(it) }
        if (intent?.action == "P2P_OFF") {
            direct?.stop()
            direct = null
        }
        return START_NOT_STICKY
    }

    override fun onBind(intent: Intent?) = null

    override fun onDestroy() {
        timer.removeCallbacksAndMessages(null)
        ble?.stop()
        direct?.stop()
        Native.stop()
        super.onDestroy()
    }
}
