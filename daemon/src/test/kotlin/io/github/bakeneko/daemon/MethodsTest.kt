package io.github.bakeneko.daemon

import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put
import kotlinx.serialization.json.buildJsonObject
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class MethodsTest {

    private val methods = Methods(DaemonLoaderContext())
    private val json = Json { ignoreUnknownKeys = true }

    @Test
    fun `ping devuelve version`() = runBlocking {
        val result = methods.invoke("ping", null).jsonObject
        assertEquals(DaemonVersion, result["version"]!!.jsonPrimitive.content)
        assertTrue(result["java"]!!.jsonPrimitive.content.isNotEmpty())
    }

    @Test
    fun `metodo inexistente lanza RpcError`() {
        var caught: RpcError? = null
        runBlocking {
            try {
                methods.invoke("no.existe", null)
            } catch (e: RpcError) {
                caught = e
            }
        }
        assertEquals(RpcCodes.METHOD_NOT_FOUND, caught!!.code)
    }

    @Test
    fun `sources lista incluye MangaDex`() = runBlocking {
        val arr = methods.invoke("sources.list", null).jsonArray
        assertTrue(arr.any { it.jsonObject["id"]!!.jsonPrimitive.content == "MANGADEX" })
        assertTrue(arr.isNotEmpty())
    }

    @Test
    fun `handleRequest serializa una respuesta ok`() = runBlocking {
        val req = json.encodeToString(
            kotlinx.serialization.json.JsonObject.serializer(),
            buildJsonObject { put("id", 1L); put("method", "ping") },
        )
        val resp = handleRequest(req, methods, json)
        val parsed = json.parseToJsonElement(resp).jsonObject
        assertEquals("1", parsed["id"]!!.jsonPrimitive.content)
        assertEquals(DaemonVersion, parsed["result"]!!.jsonObject["version"]!!.jsonPrimitive.content)
    }

    @Test
    fun `handleRequest devuelve error de parse para JSON invalido`() = runBlocking {
        val resp = handleRequest("{ no es json", methods, json)
        val parsed = json.parseToJsonElement(resp).jsonObject
        assertEquals(RpcCodes.PARSE_ERROR, parsed["error"]!!.jsonObject["code"]!!.jsonPrimitive.content.toInt())
    }
}