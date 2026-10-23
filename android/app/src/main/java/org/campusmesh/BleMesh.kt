package org.campusmesh

import android.annotation.SuppressLint
import android.bluetooth.*
import android.bluetooth.le.*
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.os.ParcelUuid
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.util.UUID
import org.json.JSONArray
import org.json.JSONObject

/**
 * Foreground BLE GATT transport. Both roles use the same length-prefixed Frame JSON. No plaintext
 * crosses GATT. 20-byte chunks work at the minimum ATT MTU. The encrypted mesh ACK, not a
 * successful GATT write, confirms delivery.
 */
@SuppressLint("MissingPermission")
class BleMesh(private val context: Context) {
    companion object {
        val SERVICE: UUID = UUID.fromString("b8554a10-6a9a-4c18-ae32-32a3bc1d0010")
        val DATA: UUID = UUID.fromString("b8554a11-6a9a-4c18-ae32-32a3bc1d0010")
        val CCC: UUID = UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")
        @Volatile var status = "蓝牙未启用"
    }

    private class Channel {
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
                            .put("op", "ingest")
                            .put("frame", JSONObject(String(raw, 4, length, Charsets.UTF_8)))
                    )
                }
                rx.reset()
            } else if (raw.size > length + 4) rx.reset()
        }

        fun prepare(frames: JSONArray) {
            if (offset < tx.size || frames.length() == 0) return
            val raw =
                frames
                    .getJSONObject(cursor++ % frames.length())
                    .toString()
                    .toByteArray(Charsets.UTF_8)
            if (raw.size > 65536) return
            tx = ByteBuffer.allocate(4 + raw.size).putInt(raw.size).put(raw).array()
            offset = 0
        }

        fun chunk() = tx.copyOfRange(offset, minOf(offset + 20, tx.size))
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
                    channels[result.device.address] = Channel()
                }
            }

            override fun onScanFailed(errorCode: Int) {
                status = "BLE 扫描失败 $errorCode"
            }
        }

    private fun updateStatus() {
        status = "BLE · ${characteristics.size+centrals.size} 条 GATT 链路"
    }

    private val clientCallback =
        object : BluetoothGattCallback() {
            override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
                handler.post {
                    if (
                        status == BluetoothGatt.GATT_SUCCESS &&
                            newState == BluetoothProfile.STATE_CONNECTED
                    )
                        gatt.discoverServices()
                    else if (
                        newState == BluetoothProfile.STATE_DISCONNECTED ||
                            status != BluetoothGatt.GATT_SUCCESS
                    ) {
                        clients.remove(gatt.device.address)
                        channels.remove(gatt.device.address)
                        characteristics.remove(gatt.device.address)
                        gatt.close()
                        updateStatus()
                    }
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
                        channel.offset += minOf(20, channel.tx.size - channel.offset)
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
                        centrals.remove(device.address)
                        updateStatus()
                    }
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
                        centrals[device.address] = device to Channel()
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
                        channel.offset += minOf(20, channel.tx.size - channel.offset)
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
                runCatching {
                        val frames =
                            Native.request(JSONObject().put("op", "outgoing").put("limit", 1))
                                .getJSONArray("data")
                        for ((address, c) in channels.toMap()) if (c.offset >= c.tx.size) {
                            c.prepare(frames)
                            clients[address]?.let { pumpClient(it, c) }
                        }
                        for ((device, c) in centrals.values.toList()) if (c.offset >= c.tx.size) {
                            c.prepare(frames)
                            pumpServer(device, c)
                        }
                    }
                    .onFailure { status = "BLE 核心不可用" }
                val interval = runCatching {
                    Native.request(JSONObject().put("op", "state")).getJSONObject("data")
                        .getJSONObject("settings").getLong("sync_interval_secs") * 1000
                }.getOrDefault(5000L)
                handler.postDelayed(this, interval)
            }
        }

    private fun startAdvertising() {
        if (!running) return
        adapter.bluetoothLeAdvertiser?.startAdvertising(
            AdvertiseSettings.Builder()
                .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_LOW_POWER)
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
        adapter.bluetoothLeScanner.startScan(
            listOf(ScanFilter.Builder().setServiceUuid(ParcelUuid(SERVICE)).build()),
            ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_POWER).build(),
            scanCallback,
        )
        handler.post(tick)
        status = "BLE 正在扫描并广播"
    }

    fun stop() {
        running = false
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
