package org.campusmesh

import org.json.JSONObject

object Native {
    init {
        System.loadLibrary("mesh_jni")
    }

    external fun open(path: ByteArray, nickname: ByteArray): Long

    external fun call(handle: Long, input: ByteArray): ByteArray

    external fun close(handle: Long)

    private var handle = 0L

    @Synchronized
    fun start(path: String, name: String) {
        if (handle == 0L) {
            handle = open(path.toByteArray(Charsets.UTF_8), name.toByteArray(Charsets.UTF_8))
            check(handle != 0L) { "通信核心启动失败，请检查端口和存储" }
        }
    }

    @Synchronized
    fun request(input: JSONObject): JSONObject {
        check(handle != 0L) { "通信服务尚未启动" }
        val response =
            JSONObject(
                String(call(handle, input.toString().toByteArray(Charsets.UTF_8)), Charsets.UTF_8)
            )
        check(response.getBoolean("ok")) { response.optString("error") }
        return response
    }

    @Synchronized
    fun stop() {
        if (handle != 0L) {
            close(handle)
            handle = 0
        }
    }
}
