package io.github.bakeneko.daemon

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull
import kotlinx.serialization.json.put
import java.net.StandardProtocolFamily
import java.net.UnixDomainSocketAddress
import java.nio.channels.AsynchronousCloseException
import java.nio.channels.ServerSocketChannel
import java.nio.channels.SocketChannel
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.attribute.PosixFilePermissions

/**
 * Punto de entrada del daemon. Acéptador bloqueante simple sobre un
 * socket Unix de dominio; una coroutina por conexión en [Dispatchers.IO]
 * lee y escribe línea a línea (JSON-RPC delimitado por `\n`).
 * Muere limpiamente al cerrarse el acéptador (SIGTERM, parent muerto).
 */
fun main() {
    val socketPath = resolveSocketPath()
    Files.createDirectories(socketPath.parent)
    Files.deleteIfExists(socketPath)

    val methods = Methods(DaemonLoaderContext())
    val json = Json { ignoreUnknownKeys = true; encodeDefaults = false }

    val server = ServerSocketChannel.open(StandardProtocolFamily.UNIX)
    try {
        server.bind(UnixDomainSocketAddress.of(socketPath))
        Files.setPosixFilePermissions(socketPath, PosixFilePermissions.fromString("rw-------"))
        System.err.println("bakeneko-daemon escuchando en $socketPath")

        val scope = CoroutineScope(SupervisorJob())
        // Receptor: acepta conexiones y lanza una coroutina por cada una.
        val acceptJob = scope.launch(Dispatchers.IO) {
            while (!server.isOpen) {} // noop
            try {
                while (true) {
                    val conn = server.accept() ?: break
                    scope.launch(Dispatchers.IO) { handleConnection(conn, methods, json) }
                }
            } catch (_: AsynchronousCloseException) {
            } catch (e: java.nio.channels.ClosedChannelException) {
            }
        }
        // Bloquea el hilo principal hasta kill/SIGTERM.
        Runtime.getRuntime().addShutdownHook(Thread {
            scope.cancel("shutdown")
            runCatching { server.close() }
            runCatching { Files.deleteIfExists(socketPath) }
        })
        runBlocking { acceptJob.join() }
    } finally {
        runCatching { server.close() }
        runCatching { Files.deleteIfExists(socketPath) }
    }
}

private fun resolveSocketPath(): Path {
    val runtime = System.getenv("XDG_RUNTIME_DIR")
        ?.takeIf { it.isNotBlank() }
        ?.let { Path.of(it) }
        ?: Path.of("/tmp").resolve("bakeneko-${getuid()}")
    return runtime.resolve("bakeneko").resolve("daemon.sock")
}

private fun getuid(): Long = ProcessHandle.current().pid()

private suspend fun handleConnection(
    channel: SocketChannel,
    methods: Methods,
    json: Json,
) {
    channel.use { ch ->
        val input = java.io.InputStreamReader(java.nio.channels.Channels.newInputStream(ch), Charsets.UTF_8).buffered()
        val output = java.io.BufferedWriter(
            java.io.OutputStreamWriter(java.nio.channels.Channels.newOutputStream(ch), Charsets.UTF_8),
        )
        var line: String? = input.readLine()
        while (line != null) {
            if (line.isNotBlank()) {
                val response = handleRequest(line.trim(), methods, json)
                output.write(response)
                output.newLine()
                output.flush()
            }
            line = input.readLine()
        }
    }
}

internal suspend fun handleRequest(rawLine: String, methods: Methods, json: Json): String =
    try {
        val raw = json.parseToJsonElement(rawLine)
        val request = raw.jsonObject
        val method = request["method"]?.jsonPrimitive?.contentOrNull
        val params = request.paramsObject()
        val id = request.rpcId()
        if (method == null) errorResponse(json, id, RpcCodes.INVALID_REQUEST, "falta method")
        else {
            val result = try {
                methods.invoke(method, params)
            } catch (e: RpcError) {
                return errorResponse(json, id, e.code, e.message ?: "error")
            } catch (kc: kotlin.coroutines.cancellation.CancellationException) {
                throw kc
            } catch (e: Throwable) {
                e.printStackTrace()
                return errorResponse(json, id, RpcCodes.INTERNAL_ERROR, e.message ?: "error interno")
            }
            okResponse(json, id, result)
        }
    } catch (e: kotlinx.serialization.SerializationException) {
        errorResponse(json, null, RpcCodes.PARSE_ERROR, "JSON inválido: ${e.message}")
    } catch (e: Throwable) {
        e.printStackTrace()
        errorResponse(json, null, RpcCodes.INTERNAL_ERROR, e.message ?: "error interno")
    }

private fun kotlinx.serialization.json.JsonElement.paramsObject(): JsonObject {
    // permitimos params ausente o como objeto. (No soportamos params como
    // array; el cliente siempre envía un objeto.)
    val v = (this as? JsonObject)?.get("params") ?: return JsonObject(emptyMap())
    return v as? JsonObject ?: JsonObject(emptyMap())
}

private fun kotlinx.serialization.json.JsonObject.rpcId(): Long? {
    val id = this["id"] ?: return null
    val p = id.jsonPrimitive
    return p.longOrNull ?: p.intOrNull?.toLong()
}

private fun okResponse(json: Json, id: Long?, result: kotlinx.serialization.json.JsonElement): String =
    json.encodeToString(
        JsonObject.serializer(),
        buildJsonObject {
            if (id != null) put("id", JsonPrimitive(id))
            put("result", result)
            put("jsonrpc", JsonPrimitive("2.0"))
        },
    )

private fun errorResponse(json: Json, id: Long?, code: Int, message: String?): String =
    json.encodeToString(
        JsonObject.serializer(),
        buildJsonObject {
            if (id != null) put("id", JsonPrimitive(id))
            put("error", buildJsonObject {
                put("code", code)
                put("message", message ?: "error")
            })
            put("jsonrpc", JsonPrimitive("2.0"))
        },
    )