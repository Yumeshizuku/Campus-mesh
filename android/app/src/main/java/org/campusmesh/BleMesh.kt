package org.campusmesh

import android.annotation.SuppressLint
import android.bluetooth.*
import android.bluetooth.le.*
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.os.ParcelUuid
import android.util.Base64
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.util.UUID
import org.json.JSONArray
import org.json.JSONObject

/**
 * Foreground BLE GATT transport. Both roles use length-prefixed CM2 compressed encrypted envelopes. No plaintext
 * crosses GATT. Chunks use the negotiated ATT MTU, with a 20-byte fallback. The encrypted mesh ACK, not a
 * successful GATT write, confirms delivery.
 */
@SuppressLint("MissingPermission")
class BleMesh(private val context: Context) {
    companion object {
        val SERVICE: UUID = UUID.fromString("b8554a10-6a9a-4c18-ae32-32a3bc1d0020")
        val DATA: UUID = UUID.fromString("b8554a11-6a9a-4c18-ae32-32a3bc1d0020")
        val CCC: UUID = UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")
        @Volatile var status = "蓝牙未启用"
    }

    private class Channel(val link: String, var rssi: Int = -80) {
        var mtu = 20
        var sent = 0
        val rx = ByteArrayOutputStream()
        var tx = ByteArray(0)
        var offset = 0
        var cursor = 0
        var last = System.currentTimeMillis()

        fun receive(bytes: ByteArray) {
            if (System.currentTimeMillis() - last > 30000) rx.reset()
            last = System.currentTimeMillis()
            if (rx.size() + bytes.size > 65540) {
                rx.reset()
                return
            }
            rx.write(bytes)
            val raw = rx.toByteArray()
            if (raw.size < 4) return
            val length = ByteBuffer.wrap(raw, 0, 4).int
            if (length !in 1..65536) {
                rx.reset()
                return
            }
            if (raw.size == length + 4) {
                runCatching {
                    Native.request(
                        JSONObject()
                            .put("op", "ingest_ble")
                            .put("link", link)
                            .put("rssi", rssi)
                            .put(
                                "wire",
                                Base64.encodeToString(
                                    raw.copyOfRange(4, 4 + length),
                                    Base64.NO_WRAP,
                                ),
                            )
                    )
                }
                rx.reset()
            } else if (raw.size > length + 4) rx.reset()
        }

        fun prepare(frames: JSONArray) {
            if (offset < tx.size || frames.length() == 0) return
            val raw = Base64.decode(frames.getString(cursor++ % frames.length()), Base64.NO_WRAP)
            if (raw.size > 65536) return
            tx = ByteBuffer.allocate(4 + raw.size).putInt(raw.size).put(raw).array()
            offset = 0
        }

        fun chunk(): ByteArray {
            sent = minOf(mtu, tx.size - offset)
            return tx.copyOfRange(offset, offset + sent)
        }
    }

    private val handler = Handler(Looper.getMainLooper())
    private val manager = context.getSystemService(BluetoothManager::class.java)
    private val adapter
        get() = manager.adapter

    private var server: BluetoothGattServer? = null
    private var characteristic: BluetoothGattCharacteristic? = null
    private val centrals = mutableMapOf<String, Pair<BluetoothDevice, Channel>>()
    private val clients = mutableMapOf<String, BluetoothGatt>()
    private val channels = mutableMapOf<String, Channel>()
    private val characteristics = mutableMapOf<String, BluetoothGattCharacteristic>()
    private var running = false
    private var scanning = false
    private var scanSince = 0L
    private var nextSync = 0L
    private var idleRounds = 0
    private var advertisingMode = AdvertiseSettings.ADVERTISE_MODE_LOW_POWER
    private val mtus = mutableMapOf<String, Int>()
    private val advertiseCallback =
        object : AdvertiseCallback() {
            override fun onStartFailure(errorCode: Int) {
                status = "BLE 广播失败 $errorCode；仍可扫描"
            }
        }
    private val scanCallback =
        object : ScanCallback() {
            override fun onScanResult(callbackType: Int, result: ScanResult) {
                handler.post {
                    channels[result.device.address]?.rssi = result.rssi
                    if (!running || clients.size >= 4 || clients.containsKey(result.device.address))
                        return@post
                    val gatt =
                        result.device.connectGatt(
                            context,
                            false,
                            clientCallback,
                            BluetoothDevice.TRANSPORT_LE,
                        )
                    clients[result.device.address] = gatt
                    channels[result.device.address] =
                        Channel("c:" + result.device.address, result.rssi)
                }
            }

            override fun onScanFailed(errorCode: Int) {
                status = "BLE 扫描失败 $errorCode"
            }
        }

    private fun updateStatus() {
        val mode =
            if (characteristics.isEmpty() && centrals.isEmpty())
                AdvertiseSettings.ADVERTISE_MODE_BALANCED
            else AdvertiseSettings.ADVERTISE_MODE_LOW_POWER
        if (mode != advertisingMode) {
            advertisingMode = mode
            adapter.bluetoothLeAdvertiser?.stopAdvertising(advertiseCallback)
            startAdvertising()
        }
        status = "BLE · ${characteristics.size+centrals.size} 条 GATT 链路"
    }

    private val clientCallback =
        object : BluetoothGattCallback() {
            override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
                handler.post {
                    if (
                        status == BluetoothGatt.GATT_SUCCESS &&
                            newState == BluetoothProfile.STATE_CONNECTED
                    ) {
                        if (!gatt.requestMtu(247)) gatt.discoverServices()
                    } else if (
                        newState == BluetoothProfile.STATE_DISCONNECTED ||
                            status != BluetoothGatt.GATT_SUCCESS
                    ) {
                        clients.remove(gatt.device.address)
                        channels.remove(gatt.device.address)?.let { dropLink(it.link) }
                        mtus.remove(gatt.device.address)
                        characteristics.remove(gatt.device.address)
                        gatt.close()
                        updateStatus()
                    }
                }
            }

            override fun onMtuChanged(gatt: BluetoothGatt, mtu: Int, status: Int) {
                handler.post {
                    if (status == BluetoothGatt.GATT_SUCCESS)
                        channels[gatt.device.address]?.mtu = (mtu - 3).coerceIn(20, 244)
                    gatt.discoverServices()
                }
            }

            override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
                handler.post {
                    val c = gatt.getService(SERVICE)?.getCharacteristic(DATA) ?: return@post
                    if (status != BluetoothGatt.GATT_SUCCESS) return@post
                    gatt.setCharacteristicNotification(c, true)
                    val descriptor = c.getDescriptor(CCC) ?: return@post
                    descriptor.value = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
                    if (!gatt.writeDescriptor(descriptor)) gatt.disconnect()
                }
            }

            override fun onDescriptorWrite(
                gatt: BluetoothGatt,
                descriptor: BluetoothGattDescriptor,
                status: Int,
            ) {
                handler.post {
                    if (status == BluetoothGatt.GATT_SUCCESS) {
                        gatt.getService(SERVICE)?.getCharacteristic(DATA)?.let {
                            characteristics[gatt.device.address] = it
                        }
                        updateStatus()
                    } else gatt.disconnect()
                }
            }

            @Deprecated("API compatibility")
            override fun onCharacteristicChanged(
                gatt: BluetoothGatt,
                c: BluetoothGattCharacteristic,
            ) {
                val raw = c.value?.clone() ?: return
                handler.post { channels[gatt.device.address]?.receive(raw) }
            }

            override fun onCharacteristicWrite(
                gatt: BluetoothGatt,
                c: BluetoothGattCharacteristic,
                status: Int,
            ) {
                handler.post {
                    if (status == BluetoothGatt.GATT_SUCCESS) {
                        val channel = channels[gatt.device.address] ?: return@post
                        channel.offset += channel.sent
                        pumpClient(gatt, channel)
                    } else gatt.disconnect()
                }
            }
        }
    private val serverCallback =
        object : BluetoothGattServerCallback() {
            override fun onConnectionStateChange(
                device: BluetoothDevice,
                status: Int,
                newState: Int,
            ) {
                handler.post {
                    if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                        centrals.remove(device.address)?.second?.let { dropLink(it.link) }
                        mtus.remove(device.address)
                        updateStatus()
                    }
                }
            }

            override fun onMtuChanged(device: BluetoothDevice, mtu: Int) {
                handler.post {
                    mtus[device.address] = (mtu - 3).coerceIn(20, 244)
                    centrals[device.address]?.second?.mtu = mtus[device.address] ?: 20
                }
            }

            override fun onServiceAdded(status: Int, service: BluetoothGattService) {
                if (status == BluetoothGatt.GATT_SUCCESS) handler.post { startAdvertising() }
            }

            override fun onDescriptorWriteRequest(
                device: BluetoothDevice,
                requestId: Int,
                descriptor: BluetoothGattDescriptor,
                preparedWrite: Boolean,
                responseNeeded: Boolean,
                offset: Int,
                value: ByteArray,
            ) {
                handler.post {
                    val valid = descriptor.uuid == CCC && !preparedWrite && offset == 0
                    if (
                        valid &&
                            value.contentEquals(
                                BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
                            ) &&
                            centrals.size < 4
                    ) {
                        centrals[device.address] =
                            device to
                                Channel("s:" + device.address).apply {
                                    mtu = mtus[device.address] ?: 20
                                }
                    } else if (valid) {
                        centrals.remove(device.address)
                    }
                    if (responseNeeded)
                        server?.sendResponse(
                            device,
                            requestId,
                            if (valid) BluetoothGatt.GATT_SUCCESS
                            else BluetoothGatt.GATT_REQUEST_NOT_SUPPORTED,
                            0,
                            null,
                        )
                    updateStatus()
                }
            }

            override fun onCharacteristicWriteRequest(
                device: BluetoothDevice,
                requestId: Int,
                c: BluetoothGattCharacteristic,
                preparedWrite: Boolean,
                responseNeeded: Boolean,
                offset: Int,
                value: ByteArray,
            ) {
                handler.post {
                    val valid =
                        c.uuid == DATA &&
                            !preparedWrite &&
                            offset == 0 &&
                            centrals.containsKey(device.address)
                    if (valid) centrals[device.address]?.second?.receive(value)
                    if (responseNeeded)
                        server?.sendResponse(
                            device,
                            requestId,
                            if (valid) BluetoothGatt.GATT_SUCCESS
                            else BluetoothGatt.GATT_REQUEST_NOT_SUPPORTED,
                            0,
                            null,
                        )
                }
            }

            override fun onNotificationSent(device: BluetoothDevice, status: Int) {
                handler.post {
                    val channel = centrals[device.address]?.second ?: return@post
                    if (status == BluetoothGatt.GATT_SUCCESS) {
                        channel.offset += channel.sent
                        pumpServer(device, channel)
                    } else {
                        server?.cancelConnection(device)
                        centrals.remove(device.address)
                    }
                }
            }
        }

    private fun pumpClient(gatt: BluetoothGatt, channel: Channel) {
        if (channel.offset >= channel.tx.size) return
        val c = characteristics[gatt.device.address] ?: return
        c.writeType = BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
        c.value = channel.chunk()
        if (!gatt.writeCharacteristic(c)) gatt.disconnect()
    }

    private fun pumpServer(device: BluetoothDevice, channel: Channel) {
        if (channel.offset >= channel.tx.size) return
        characteristic?.let {
            it.value = channel.chunk()
            if (server?.notifyCharacteristicChanged(device, it, false) != true) {
                server?.cancelConnection(device)
                centrals.remove(device.address)
            }
        }
    }

    private val tick =
        object : Runnable {
            override fun run() {
                if (!running) return
                updateScanWindow()
                if (System.currentTimeMillis() < nextSync) {
                    handler.postDelayed(this, 1000)
                    return
                }
                var offered = 0
                runCatching {
                        for ((address, c) in channels.toMap()) if (c.offset >= c.tx.size) {
                            val frames =
                                Native.request(
                                        JSONObject()
                                            .put("op", "outgoing_ble")
                                            .put("link", c.link)
                                            .put("rssi", c.rssi)
                                            .put("limit", 1)
                                    )
                                    .getJSONArray("data")
                            offered += frames.length()
                            c.prepare(frames)
                            clients[address]?.let { pumpClient(it, c) }
                        }
                        for ((device, c) in centrals.values.toList()) if (c.offset >= c.tx.size) {
                            val frames =
                                Native.request(
                                        JSONObject()
                                            .put("op", "outgoing_ble")
                                            .put("link", c.link)
                                            .put("rssi", c.rssi)
                                            .put("limit", 1)
                                    )
                                    .getJSONArray("data")
                            offered += frames.length()
                            c.prepare(frames)
                            pumpServer(device, c)
                        }
                    }
                    .onFailure { status = "BLE 核心不可用" }
                val interval =
                    runCatching {
                            Native.request(JSONObject().put("op", "state"))
                                .getJSONObject("data")
                                .getJSONObject("settings")
                                .getLong("sync_interval_secs") * 1000
                        }
                        .getOrDefault(5000L)
                idleRounds = if (offered == 0) minOf(idleRounds + 1, 4) else 0
                nextSync = System.currentTimeMillis() + minOf(60000L, interval * (1 + idleRounds))
                handler.postDelayed(this, 1000)
            }
        }

    private fun dropLink(link: String) {
        runCatching { Native.request(JSONObject().put("op", "link_down").put("link", link)) }
    }

    private fun setScanning(enabled: Boolean) {
        if (scanning == enabled) return
        if (enabled)
            adapter.bluetoothLeScanner?.startScan(
                listOf(ScanFilter.Builder().setServiceUuid(ParcelUuid(SERVICE)).build()),
                ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_POWER).build(),
                scanCallback,
            )
        else adapter.bluetoothLeScanner?.stopScan(scanCallback)
        scanning = enabled
        scanSince = System.currentTimeMillis()
    }

    private fun updateScanWindow() {
        val elapsed = System.currentTimeMillis() - scanSince
        // Connected: scan 10s, sleep 20s. Isolated: scan 10s, sleep 5s.
        val pause = if (characteristics.isEmpty() && centrals.isEmpty()) 5000 else 20000
        if (elapsed >= if (scanning) 10000 else pause) setScanning(!scanning)
    }

    private fun startAdvertising() {
        if (!running) return
        adapter.bluetoothLeAdvertiser?.startAdvertising(
            AdvertiseSettings.Builder()
                .setAdvertiseMode(advertisingMode)
                .setConnectable(true)
                .setTimeout(0)
                .build(),
            AdvertiseData.Builder().addServiceUuid(ParcelUuid(SERVICE)).build(),
            advertiseCallback,
        )
    }

    fun start() {
        if (running) return
        check(adapter != null && adapter.isEnabled) { "请先开启系统蓝牙" }
        running = true
        nextSync = 0L
        idleRounds = 0
        advertisingMode = AdvertiseSettings.ADVERTISE_MODE_BALANCED
        server = manager.openGattServer(context, serverCallback)
        val service = BluetoothGattService(SERVICE, BluetoothGattService.SERVICE_TYPE_PRIMARY)
        characteristic =
            BluetoothGattCharacteristic(
                DATA,
                BluetoothGattCharacteristic.PROPERTY_WRITE or
                    BluetoothGattCharacteristic.PROPERTY_NOTIFY,
                BluetoothGattCharacteristic.PERMISSION_WRITE,
            )
        characteristic!!.addDescriptor(
            BluetoothGattDescriptor(
                CCC,
                BluetoothGattDescriptor.PERMISSION_READ or BluetoothGattDescriptor.PERMISSION_WRITE,
            )
        )
        service.addCharacteristic(characteristic)
        server?.addService(service)
        setScanning(true)
        handler.post(tick)
        status = "BLE 正在扫描并广播"
    }

    fun stop() {
        running = false
        for (c in channels.values) dropLink(c.link)
        for ((_, c) in centrals.values) dropLink(c.link)
        scanning = false
        mtus.clear()
        handler.removeCallbacks(tick)
        runCatching {
            adapter.bluetoothLeScanner?.stopScan(scanCallback)
            adapter.bluetoothLeAdvertiser?.stopAdvertising(advertiseCallback)
        }
        for (g in clients.values) {
            runCatching {
                g.disconnect()
                g.close()
            }
        }
        clients.clear()
        channels.clear()
        characteristics.clear()
        centrals.clear()
        server?.close()
        server = null
        status = "蓝牙已停止"
    }
}
