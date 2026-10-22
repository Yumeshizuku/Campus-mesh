package org.campusmesh

import android.annotation.SuppressLint
import android.content.*
import android.net.wifi.p2p.*
import android.os.Looper
import org.json.JSONObject

/**
 * Authorized Android Wi-Fi P2P group creation; system pairing consent is retained. Android clients
 * initiate ordinary mesh TCP sync to the group owner. iOS cannot join via an equivalent public
 * Wi-Fi Direct API; use BLE or normal LAN.
 */
@SuppressLint("MissingPermission")
class WifiDirectMesh(private val context: Context) {
    companion object {
        @Volatile var status = "Wi-Fi Direct 未启用"
        @Volatile var peers: List<WifiP2pDevice> = emptyList()
    }

    private val manager = context.getSystemService(Context.WIFI_P2P_SERVICE) as? WifiP2pManager
    private var channel: WifiP2pManager.Channel? = null
    private var registered = false
    private val receiver =
        object : BroadcastReceiver() {
            override fun onReceive(c: Context, i: Intent) {
                when (i.action) {
                    WifiP2pManager.WIFI_P2P_PEERS_CHANGED_ACTION ->
                        channel?.let {
                            manager?.requestPeers(it) { list ->
                                peers = list.deviceList.toList()
                                status = "Wi-Fi Direct · ${peers.size} 个设备"
                            }
                        }
                    WifiP2pManager.WIFI_P2P_CONNECTION_CHANGED_ACTION ->
                        channel?.let {
                            manager?.requestConnectionInfo(it) { info ->
                                if (info.groupFormed) {
                                    status =
                                        if (info.isGroupOwner) "Wi-Fi Direct 群主 · 等待客户端同步"
                                        else "Wi-Fi Direct 已连接"
                                    if (!info.isGroupOwner)
                                        info.groupOwnerAddress?.hostAddress?.let { address ->
                                            runCatching {
                                                Native.request(
                                                    JSONObject()
                                                        .put("op", "peer")
                                                        .put("address", "$address:37078")
                                                )
                                            }
                                        }
                                } else status = "Wi-Fi Direct 等待连接"
                            }
                        }
                    WifiP2pManager.WIFI_P2P_STATE_CHANGED_ACTION ->
                        if (
                            i.getIntExtra(WifiP2pManager.EXTRA_WIFI_STATE, -1) !=
                                WifiP2pManager.WIFI_P2P_STATE_ENABLED
                        )
                            status = "请开启系统 Wi-Fi"
                }
            }
        }

    private fun callback(success: String) =
        object : WifiP2pManager.ActionListener {
            override fun onSuccess() {
                status = success
            }

            override fun onFailure(reason: Int) {
                status = "Wi-Fi Direct 失败 $reason（检查权限、定位开关及设备支持）"
            }
        }

    fun discover() {
        check(manager != null) { "设备不支持 Wi-Fi Direct" }
        if (channel == null)
            channel =
                manager.initialize(context, Looper.getMainLooper()) {
                    status = "Wi-Fi Direct 服务断开"
                    channel = null
                }
        if (!registered) {
            val filter =
                IntentFilter().apply {
                    addAction(WifiP2pManager.WIFI_P2P_PEERS_CHANGED_ACTION)
                    addAction(WifiP2pManager.WIFI_P2P_CONNECTION_CHANGED_ACTION)
                    addAction(WifiP2pManager.WIFI_P2P_STATE_CHANGED_ACTION)
                }
            if (android.os.Build.VERSION.SDK_INT >= 33)
                context.registerReceiver(receiver, filter, Context.RECEIVER_EXPORTED)
            else context.registerReceiver(receiver, filter)
            registered = true
        }
        channel?.let { manager.discoverPeers(it, callback("Wi-Fi Direct 搜索中…")) }
    }

    fun connect(address: String) {
        channel?.let {
            manager?.connect(
                it,
                WifiP2pConfig().apply { deviceAddress = address },
                callback("等待系统配对确认"),
            )
        }
    }

    fun stop() {
        channel?.let {
            manager?.stopPeerDiscovery(it, callback("Wi-Fi Direct 已停止"))
            manager?.removeGroup(it, callback("Wi-Fi Direct 已断开"))
            if (android.os.Build.VERSION.SDK_INT >= 27) it.close()
        }
        channel = null
        if (registered) {
            context.unregisterReceiver(receiver)
            registered = false
        }
        peers = emptyList()
    }
}
